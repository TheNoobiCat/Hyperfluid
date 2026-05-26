// === Network Bridge ===
//
// Bridges tokio channels (BFT driver) to/from peer-specific channels
// for multi-validator networking.
//
// Wire format:
//   Vote      [0x01] [8B height LE] [4B round LE] [1B value_type] [0 or 32B hash] [1B vote_type] [32B addr] [4B sig_len] [sig bytes]
//   Proposal  [0x02] [8B height LE] [4B round LE] [32B value_hash] [4B pol_round LE] [32B addr] [4B sig_len] [sig bytes]
//
// Source: ADR-0018, docs/specs/protocol/consensus-spec.md

use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use arc_malachitebft_core_types::{
    Height, NilOrVal, Round, SignedMessage, Value as ValueTrait, VoteType,
};

use crate::malachite::{
    Address32, BlockHeight, HyperfluidContext, HyperfluidProposal, HyperfluidVote,
    MlDsa65Signature, ValueHash,
};
use crate::malachite_consensus::ConsensusNetworkMsg;

/// Network bridge that receives consensus messages from the BFT driver
/// and broadcasts them to all peer channels, and receives messages from
/// peer channels and feeds them back into the consensus loop.
pub struct NetworkBridge {
    /// Channel to forward consensus messages to external consumers
    /// (e.g., P2P network integration layer)
    pub outgoing: mpsc::UnboundedSender<ConsensusNetworkMsg>,
    /// Peer-specific channels for broadcast
    pub peers: Vec<mpsc::UnboundedSender<Vec<u8>>>,
}

// ---------------------------------------------------------------------------
// Wire encoding helpers
// ---------------------------------------------------------------------------

/// Encode a SignedVote into bytes.
pub fn encode_vote(vote: &SignedMessage<HyperfluidContext, HyperfluidVote>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(128);
    // height
    buf.extend_from_slice(&vote.message.height.as_u64().to_le_bytes());
    // round
    buf.extend_from_slice(&vote.message.round.as_u32().unwrap_or(0).to_le_bytes());
    // value_id
    match &vote.message.value_id {
        NilOrVal::Val(hash) => {
            buf.push(1u8);
            buf.extend_from_slice(&hash.0);
        }
        NilOrVal::Nil => {
            buf.push(0u8);
        }
    }
    // vote_type
    buf.push(match vote.message.vote_type {
        VoteType::Prevote => 1u8,
        VoteType::Precommit => 2u8,
    });
    // validator_addr
    buf.extend_from_slice(&vote.message.validator_addr.0);
    // signature: length-prefixed
    let sig_bytes = &vote.signature.0;
    buf.extend_from_slice(&(sig_bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(sig_bytes);
    buf
}

/// Decode bytes into a SignedVote. Returns None on failure.
pub fn decode_vote(bytes: &[u8]) -> Option<SignedMessage<HyperfluidContext, HyperfluidVote>> {
    let mut cursor = 0usize;

    // height (8 bytes)
    if cursor + 8 > bytes.len() {
        return None;
    }
    let height = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?);
    cursor += 8;

    // round (4 bytes)
    if cursor + 4 > bytes.len() {
        return None;
    }
    let round_u32 = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?);
    cursor += 4;

    // value_id type (1 byte)
    if cursor + 1 > bytes.len() {
        return None;
    }
    let value_type = bytes[cursor];
    cursor += 1;

    let value_id = if value_type == 1 {
        if cursor + 32 > bytes.len() {
            return None;
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[cursor..cursor + 32]);
        cursor += 32;
        NilOrVal::Val(ValueHash(hash))
    } else {
        NilOrVal::Nil
    };

    // vote_type (1 byte)
    if cursor + 1 > bytes.len() {
        return None;
    }
    let vote_type = match bytes[cursor] {
        1 => VoteType::Prevote,
        2 => VoteType::Precommit,
        _ => return None,
    };
    cursor += 1;

    // validator_addr (32 bytes)
    if cursor + 32 > bytes.len() {
        return None;
    }
    let mut addr = [0u8; 32];
    addr.copy_from_slice(&bytes[cursor..cursor + 32]);
    cursor += 32;

    // signature length (4 bytes)
    if cursor + 4 > bytes.len() {
        return None;
    }
    let sig_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?);
    cursor += 4;

    // signature bytes
    if cursor + sig_len as usize > bytes.len() {
        return None;
    }
    let sig_bytes = bytes[cursor..cursor + sig_len as usize].to_vec();

    let vote = HyperfluidVote {
        height: BlockHeight::new(height),
        round: Round::new(round_u32),
        value_id,
        vote_type,
        validator_addr: Address32::new(addr),
    };

    Some(SignedMessage::new(vote, MlDsa65Signature(sig_bytes)))
}

/// Encode a SignedProposal into bytes.
fn encode_proposal(proposal: &SignedMessage<HyperfluidContext, HyperfluidProposal>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(128);
    // height
    buf.extend_from_slice(&proposal.message.height.as_u64().to_le_bytes());
    // round
    buf.extend_from_slice(&proposal.message.round.as_u32().unwrap_or(0).to_le_bytes());
    // value hash
    let value_hash = ValueTrait::id(&proposal.message.value).0;
    buf.extend_from_slice(&value_hash);
    // pol_round
    buf.extend_from_slice(&proposal.message.pol_round.as_u32().unwrap_or(0).to_le_bytes());
    // proposer_addr
    buf.extend_from_slice(&proposal.message.proposer_addr.0);
    // signature: length-prefixed
    let sig_bytes = &proposal.signature.0;
    buf.extend_from_slice(&(sig_bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(sig_bytes);
    buf
}

/// Decode bytes into a SignedProposal. Returns None on failure.
pub fn decode_proposal(
    bytes: &[u8],
) -> Option<SignedMessage<HyperfluidContext, HyperfluidProposal>> {
    let mut cursor = 0usize;

    // height (8 bytes)
    if cursor + 8 > bytes.len() {
        return None;
    }
    let height = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?);
    cursor += 8;

    // round (4 bytes)
    if cursor + 4 > bytes.len() {
        return None;
    }
    let round_u32 = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?);
    cursor += 4;

    // value hash (32 bytes)
    if cursor + 32 > bytes.len() {
        return None;
    }
    let mut value_hash = [0u8; 32];
    value_hash.copy_from_slice(&bytes[cursor..cursor + 32]);
    cursor += 32;

    // pol_round (4 bytes)
    if cursor + 4 > bytes.len() {
        return None;
    }
    let pol_round_u32 = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?);
    cursor += 4;

    // proposer_addr (32 bytes)
    if cursor + 32 > bytes.len() {
        return None;
    }
    let mut proposer_addr = [0u8; 32];
    proposer_addr.copy_from_slice(&bytes[cursor..cursor + 32]);
    cursor += 32;

    // signature length (4 bytes)
    if cursor + 4 > bytes.len() {
        return None;
    }
    let sig_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?);
    cursor += 4;

    // signature bytes
    if cursor + sig_len as usize > bytes.len() {
        return None;
    }
    let sig_bytes = bytes[cursor..cursor + sig_len as usize].to_vec();

    // Derive parent_hash, state_root, and transaction_root from the value_hash
    // using domain separation. The wire format only carries the block hash
    // (value_hash), not the full block data, so we derive the roots
    // deterministically from the hash itself.
    let parent_hash =
        crate::malachite_consensus::sha3_256_hash(&[&value_hash[..], &[0x00]].concat());
    let state_root =
        crate::malachite_consensus::sha3_256_hash(&[&value_hash[..], &[0x01]].concat());
    let transaction_root =
        crate::malachite_consensus::sha3_256_hash(&[&value_hash[..], &[0x02]].concat());

    let block = crate::types::Block {
        header: crate::types::BlockHeader {
            height,
            parent_hash,
            state_root,
            transaction_root,
            committee_id: 0,
            proposer_id: proposer_addr,
            timestamp: 0,
            epoch: height / 100,
        },
        transactions: vec![],
    };
    let block_value = crate::malachite::BlockValue::with_hash(block, ValueHash(value_hash));

    let proposal = HyperfluidProposal {
        height: BlockHeight::new(height),
        round: Round::new(round_u32),
        value: block_value,
        pol_round: Round::new(pol_round_u32),
        proposer_addr: Address32::new(proposer_addr),
    };

    Some(SignedMessage::new(proposal, MlDsa65Signature(sig_bytes)))
}

// ---------------------------------------------------------------------------
// Sender / Receiver tasks
// ---------------------------------------------------------------------------

/// Spawn a sender task that reads ConsensusNetworkMsg from `rx`,
/// serializes each message with the wire format, and broadcasts the
/// resulting bytes to ALL peer channels.
///
/// Task exits when `rx` is closed (all senders dropped).
pub fn run_sender(
    bridge: Arc<Mutex<NetworkBridge>>,
    mut rx: mpsc::UnboundedReceiver<ConsensusNetworkMsg>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let serialized = match &msg {
                ConsensusNetworkMsg::Vote(vote) => {
                    let mut buf = vec![0x01u8];
                    buf.extend(encode_vote(vote));
                    buf
                }
                ConsensusNetworkMsg::Proposal(proposal) => {
                    let mut buf = vec![0x02u8];
                    buf.extend(encode_proposal(proposal));
                    buf
                }
            };

            // Lock bridge and broadcast to all peers
            let peers = match bridge.lock() {
                Ok(b) => b.peers.clone(),
                Err(e) => {
                    tracing::error!("NetworkBridge: lock error in run_sender: {:?}", e);
                    continue;
                }
            };

            for peer in &peers {
                let _ = peer.send(serialized.clone());
            }
        }

        tracing::debug!("NetworkBridge: sender task exited (rx closed)");
    })
}

/// Spawn receiver tasks that listen on each peer receiver channel,
/// decode incoming bytes back into ConsensusNetworkMsg, and forward
/// them into the shared `tx` channel.
///
/// Each peer receiver gets its own spawned subtask that runs until
/// its specific receiver is closed. The overall task waits for all
/// subtasks to finish.
pub fn run_receiver(
    peer_rxs: Vec<mpsc::UnboundedReceiver<Vec<u8>>>,
    tx: mpsc::UnboundedSender<ConsensusNetworkMsg>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut handles = Vec::with_capacity(peer_rxs.len());

        for mut rx in peer_rxs {
            let tx = tx.clone();
            handles.push(tokio::spawn(async move {
                while let Some(bytes) = rx.recv().await {
                    if bytes.is_empty() {
                        tracing::warn!("NetworkBridge: received empty message from peer");
                        continue;
                    }

                    let msg = match bytes[0] {
                        0x01 => match decode_vote(&bytes[1..]) {
                            Some(vote) => ConsensusNetworkMsg::Vote(vote),
                            None => {
                                tracing::warn!(
                                    "NetworkBridge: failed to decode SignedVote ({} bytes)",
                                    bytes.len().saturating_sub(1),
                                );
                                continue;
                            }
                        },
                        0x02 => match decode_proposal(&bytes[1..]) {
                            Some(proposal) => ConsensusNetworkMsg::Proposal(proposal),
                            None => {
                                tracing::warn!(
                                    "NetworkBridge: failed to decode SignedProposal ({} bytes)",
                                    bytes.len().saturating_sub(1),
                                );
                                continue;
                            }
                        },
                        tag => {
                            tracing::warn!("NetworkBridge: unknown message tag: 0x{:02x}", tag,);
                            continue;
                        }
                    };

                    let _ = tx.send(msg);
                }
            }));
        }

        // Wait for all peer subtasks to complete
        for h in handles {
            let _ = h.await;
        }

        tracing::debug!("NetworkBridge: receiver task exited (all peer rxs closed)");
    })
}

/// Create a new peer channel pair for use with the network bridge.
pub fn new_peer_channel() -> (mpsc::UnboundedSender<Vec<u8>>, mpsc::UnboundedReceiver<Vec<u8>>) {
    mpsc::unbounded_channel()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::malachite::{BlockValue, HyperfluidVote, MlDsa65Signature, ValueHash};
    use crate::types::{Block, BlockHeader};
    use std::time::Duration;

    fn dummy_vote() -> SignedMessage<HyperfluidContext, HyperfluidVote> {
        let vote = HyperfluidVote {
            height: BlockHeight::new(1),
            round: Round::ZERO,
            value_id: NilOrVal::Val(ValueHash([0x42u8; 32])),
            vote_type: VoteType::Prevote,
            validator_addr: Address32::new([0xAAu8; 32]),
        };
        SignedMessage::new(vote, MlDsa65Signature(vec![0xBBu8; 100]))
    }

    fn dummy_proposal() -> SignedMessage<HyperfluidContext, HyperfluidProposal> {
        let block = Block {
            header: BlockHeader {
                height: 1,
                parent_hash: [0u8; 32],
                state_root: [1u8; 32],
                transaction_root: [2u8; 32],
                committee_id: 0,
                proposer_id: [3u8; 32],
                timestamp: 2,
                epoch: 0,
            },
            transactions: vec![],
        };
        let value = BlockValue::new(block);
        let proposal = HyperfluidProposal {
            height: BlockHeight::new(1),
            round: Round::ZERO,
            value,
            pol_round: Round::Nil,
            proposer_addr: Address32::new([0xCCu8; 32]),
        };
        SignedMessage::new(proposal, MlDsa65Signature(vec![0xDDu8; 100]))
    }

    /// Verify that Vote and Proposal messages roundtrip through the
    /// serialization format.
    #[test]
    fn conforms_to_consensus_spec_section7_serialization_roundtrip() {
        let vote = dummy_vote();
        let proposal = dummy_proposal();

        // Vote roundtrip
        let vote_encoded = encode_vote(&vote);
        assert!(!vote_encoded.is_empty(), "vote encoding must produce data");
        let decoded_vote = decode_vote(&vote_encoded).expect("vote decode must succeed");
        assert_eq!(decoded_vote.message.height.as_u64(), vote.message.height.as_u64());
        assert_eq!(decoded_vote.message.round.as_u32(), vote.message.round.as_u32());
        assert_eq!(decoded_vote.message.validator_addr.0, vote.message.validator_addr.0);

        // Proposal roundtrip
        let prop_encoded = encode_proposal(&proposal);
        assert!(!prop_encoded.is_empty(), "proposal encoding must produce data");
        let decoded_proposal =
            decode_proposal(&prop_encoded).expect("proposal decode must succeed");
        assert_eq!(decoded_proposal.message.height.as_u64(), proposal.message.height.as_u64());
        assert_eq!(decoded_proposal.message.round.as_u32(), proposal.message.round.as_u32());
        assert_eq!(decoded_proposal.message.proposer_addr.0, proposal.message.proposer_addr.0);
    }

    /// Verify that a sender task correctly forwards a message to all peer
    /// channels, and a receiver task correctly decodes and forwards back.
    #[tokio::test]
    async fn conforms_to_consensus_spec_section7_sender_receiver_flow() {
        let vote = dummy_vote();
        let msg = ConsensusNetworkMsg::Vote(vote.clone());

        // Create peer channels
        let (peer1_tx, peer1_rx) = new_peer_channel();
        let (peer2_tx, peer2_rx) = new_peer_channel();

        // Build the bridge
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let bridge = Arc::new(Mutex::new(NetworkBridge {
            outgoing: outgoing_tx.clone(),
            peers: vec![peer1_tx, peer2_tx],
        }));

        // Spawn sender and receiver
        let _sender_handle = run_sender(Arc::clone(&bridge), outgoing_rx);
        let (result_tx, mut result_rx) = mpsc::unbounded_channel();
        let _receiver_handle = run_receiver(vec![peer1_rx, peer2_rx], result_tx);

        // Send the message through the bridge's outgoing channel
        outgoing_tx.send(msg.clone()).expect("send into bridge must succeed");

        // Allow a small amount of time for the message to travel
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Both peers should have received the message (decoded from result_rx)
        let mut received_count = 0u32;
        while let Ok(received) = result_rx.try_recv() {
            match (&msg, &received) {
                (ConsensusNetworkMsg::Vote(orig), ConsensusNetworkMsg::Vote(dec)) => {
                    assert_eq!(orig.message.height.as_u64(), dec.message.height.as_u64());
                    assert_eq!(orig.message.round.as_u32(), dec.message.round.as_u32());
                }
                _ => panic!("Expected Vote message"),
            }
            received_count += 1;
        }

        assert_eq!(received_count, 2, "Both peer channels must receive the decoded message");
    }

    /// Verify that decode_vote with Nil value works correctly.
    #[test]
    fn conforms_to_consensus_spec_section7_nil_vote_roundtrip() {
        let vote = HyperfluidVote {
            height: BlockHeight::new(42),
            round: Round::new(7),
            value_id: NilOrVal::Nil,
            vote_type: VoteType::Precommit,
            validator_addr: Address32::new([0xFFu8; 32]),
        };
        let signed = SignedMessage::new(vote, MlDsa65Signature(vec![0xEEu8; 64]));

        let encoded = encode_vote(&signed);
        let decoded = decode_vote(&encoded).expect("nil vote decode must succeed");
        assert!(decoded.message.value_id.is_nil());
        assert_eq!(decoded.message.vote_type, VoteType::Precommit);
        assert_eq!(decoded.message.height.as_u64(), 42);
    }

    /// Verify that decode_vote rejects truncated input.
    #[test]
    fn conforms_to_consensus_spec_section7_decode_vote_rejects_truncated() {
        let truncated = vec![0x01u8, 0x00, 0x00, 0x00]; // way too short
        assert!(decode_vote(&truncated).is_none());
    }
}
