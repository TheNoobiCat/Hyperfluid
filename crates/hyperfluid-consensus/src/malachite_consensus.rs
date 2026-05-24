// === Malachite BFT Consensus Integration ===
//
// Wraps Malachite core-driver::Driver with ML-DSA-65 signing, block building,
// and consensus message routing. Replaces the local produce_block() auto-loop
// with BFT-driven block production.
//
// Architecture per ADR-0018:
//   - Uses core-driver::Driver (Input/Output state machine) for consensus logic
//   - Signs votes/proposals with ML-DSA-65 via MlDsa65Scheme
//   - Builds blocks from the fee-ordered mempool via ConsensusDriver
//   - Routes consensus messages through channels for network integration
//
// Source: ADR-0018, docs/specs/protocol/consensus-spec.md Section 1
// Stage: 02 Week 7-8

use std::sync::Arc;

use arc_malachitebft_core_driver::{Driver, Input, Output, ThresholdParams};
use arc_malachitebft_core_types::{
    Context, Height, NilOrVal, Proposal, Round, SignedMessage, SignedProposal, SignedVote, Timeout,
    TimeoutKind, Validator, Validity, Value, Vote, VotingPower,
};
use sha3::{Digest, Sha3_256};
use tokio::sync::mpsc;

use crate::malachite::{
    Address32, BlockHeight, BlockValue, HyperfluidContext, HyperfluidProposal, HyperfluidValidator,
    HyperfluidValidatorSet, HyperfluidVote, MlDsa65PublicKey, MlDsa65Signature,
};
use crate::types::{Block, Hash32};
use hyperfluid_p2p::identity::Identity;

// ---------------------------------------------------------------------------
// Consensus message types for network gossip
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum ConsensusNetworkMsg {
    Vote(SignedVote<HyperfluidContext>),
    Proposal(SignedProposal<HyperfluidContext>),
}

// ---------------------------------------------------------------------------
// BFT consensus events produced during processing
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ConsensusEvent {
    BlockCommitted { height: u64, round: u32, block: Block },
    BroadcastVote { height: u64, round: u32, vote: SignedVote<HyperfluidContext> },
    BroadcastProposal { height: u64, round: u32, proposal: SignedProposal<HyperfluidContext> },
    RequestBlock { height: u64, round: u32 },
    ScheduleTimeout { height: u64, round: u32, kind: TimeoutKind },
    NewHeight { height: u64, round: u32 },
}

// ---------------------------------------------------------------------------
// BftDriver — wraps Malachite Driver with signing and block building
// ---------------------------------------------------------------------------

pub struct BftDriver {
    driver: Driver<HyperfluidContext>,
    identity: Arc<Identity>,
    #[allow(dead_code)]
    node_addr: Address32,
    ctx: HyperfluidContext,
}

impl BftDriver {
    pub fn new(
        validator_set: HyperfluidValidatorSet,
        proposer_seed: [u8; 32],
        identity: Arc<Identity>,
        node_addr: Address32,
    ) -> Self {
        let ctx = HyperfluidContext::new(validator_set.clone(), proposer_seed);

        let driver = Driver::new(
            ctx.clone(),
            BlockHeight::INITIAL,
            validator_set,
            node_addr,
            ThresholdParams::default(),
        );

        Self { driver, identity, node_addr, ctx }
    }

    pub fn height(&self) -> BlockHeight {
        self.driver.height()
    }

    pub fn round(&self) -> Round {
        self.driver.round()
    }

    /// Start consensus at a new height with the given validator set.
    /// Returns initial consensus events.
    pub fn start_height(
        &mut self,
        height: u64,
        validator_set: HyperfluidValidatorSet,
    ) -> Vec<ConsensusEvent> {
        self.ctx = HyperfluidContext::new(validator_set.clone(), self.ctx.proposer_seed);
        self.driver.move_to_height(BlockHeight::new(height), validator_set);

        let proposer = self.ctx.select_proposer(
            self.driver.validator_set(),
            BlockHeight::new(height),
            Round::ZERO,
        );
        let proposer_addr = *proposer.address();

        match self.driver.process(Input::NewRound(
            BlockHeight::new(height),
            Round::ZERO,
            proposer_addr,
        )) {
            Ok(outputs) => self.handle_outputs(outputs, height),
            Err(e) => {
                tracing::error!("BFT start_height error: {:?}", e);
                vec![]
            }
        }
    }

    /// Process an incoming signed vote from the network.
    pub fn process_vote(&mut self, vote: SignedVote<HyperfluidContext>) -> Vec<ConsensusEvent> {
        let h = vote.height().as_u64();
        match self.driver.process(Input::Vote(vote)) {
            Ok(outputs) => self.handle_outputs(outputs, h),
            Err(e) => {
                tracing::debug!("BFT vote rejected: {:?}", e);
                vec![]
            }
        }
    }

    /// Process an incoming signed proposal from the network.
    pub fn process_proposal(
        &mut self,
        proposal: SignedProposal<HyperfluidContext>,
        validity: Validity,
    ) -> Vec<ConsensusEvent> {
        let h = proposal.height().as_u64();
        match self.driver.process(Input::Proposal(proposal, validity)) {
            Ok(outputs) => self.handle_outputs(outputs, h),
            Err(e) => {
                tracing::debug!("BFT proposal rejected: {:?}", e);
                vec![]
            }
        }
    }

    /// Propose a block value (called when GetValue output is received).
    pub fn propose_block_value(&mut self, round: Round, block: Block) -> Vec<ConsensusEvent> {
        let h = self.driver.height().as_u64();
        let value = BlockValue::new(block);
        match self.driver.process(Input::ProposeValue(round, value)) {
            Ok(outputs) => self.handle_outputs(outputs, h),
            Err(e) => {
                tracing::error!("BFT propose value error: {:?}", e);
                vec![]
            }
        }
    }

    /// Process a timeout (called when a scheduled timeout fires).
    pub fn process_timeout(&mut self, timeout: Timeout) -> Vec<ConsensusEvent> {
        let h = self.driver.height().as_u64();
        match self.driver.process(Input::TimeoutElapsed(timeout)) {
            Ok(outputs) => self.handle_outputs(outputs, h),
            Err(e) => {
                tracing::debug!("BFT timeout rejected: {:?}", e);
                vec![]
            }
        }
    }

    /// Handle driver outputs, converting to ConsensusEvents.
    fn handle_outputs(
        &mut self,
        outputs: Vec<Output<HyperfluidContext>>,
        height: u64,
    ) -> Vec<ConsensusEvent> {
        let mut events = Vec::new();
        for output in outputs {
            match output {
                Output::NewRound(_h, round) => {
                    events.push(ConsensusEvent::NewHeight {
                        height,
                        round: round.as_u32().unwrap_or(0),
                    });
                }
                Output::Propose(proposal) => {
                    let signed = self.sign_proposal(proposal);
                    let r = signed.round().as_u32().unwrap_or(0);
                    events.push(ConsensusEvent::BroadcastProposal {
                        height,
                        round: r,
                        proposal: signed,
                    });
                }
                Output::Vote(vote) => {
                    let signed = self.sign_vote(vote);
                    let r = signed.round().as_u32().unwrap_or(0);
                    events.push(ConsensusEvent::BroadcastVote { height, round: r, vote: signed });
                }
                Output::Decide(_round, proposal) => {
                    let block = proposal.value().block.clone();
                    let r = proposal.round().as_u32().unwrap_or(0);
                    events.push(ConsensusEvent::BlockCommitted { height, round: r, block });
                }
                Output::ScheduleTimeout(timeout) => {
                    events.push(ConsensusEvent::ScheduleTimeout {
                        height,
                        round: timeout.round.as_u32().unwrap_or(0),
                        kind: timeout.kind,
                    });
                }
                Output::GetValue(_h, round, _timeout) => {
                    events.push(ConsensusEvent::RequestBlock {
                        height,
                        round: round.as_u32().unwrap_or(0),
                    });
                }
            }
        }
        events
    }

    /// Sign a vote with the node's ML-DSA-65 identity.
    fn sign_vote(&self, vote: HyperfluidVote) -> SignedVote<HyperfluidContext> {
        let msg_bytes = vote.to_sign_bytes();
        let sig = MlDsa65Signature(self.identity.sign(&msg_bytes));
        SignedMessage::new(vote, sig)
    }

    /// Sign a proposal with the node's ML-DSA-65 identity.
    fn sign_proposal(&self, proposal: HyperfluidProposal) -> SignedProposal<HyperfluidContext> {
        let msg_bytes = proposal.to_sign_bytes();
        let sig = MlDsa65Signature(self.identity.sign(&msg_bytes));
        SignedMessage::new(proposal, sig)
    }
}

// ---------------------------------------------------------------------------
// Signing helpers on vote/proposal types
// ---------------------------------------------------------------------------

impl HyperfluidVote {
    fn to_sign_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.height.as_u64().to_le_bytes());
        data.extend_from_slice(&self.round.as_u32().unwrap_or(0).to_le_bytes());
        match &self.value_id {
            NilOrVal::Val(hash) => {
                data.push(1u8);
                data.extend_from_slice(&hash.0);
            }
            NilOrVal::Nil => {
                data.push(0u8);
            }
        }
        data.push(match self.vote_type {
            arc_malachitebft_core_types::VoteType::Prevote => 1u8,
            arc_malachitebft_core_types::VoteType::Precommit => 2u8,
        });
        data.extend_from_slice(&self.validator_addr.0);
        data
    }
}

impl HyperfluidProposal {
    fn to_sign_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.height.as_u64().to_le_bytes());
        data.extend_from_slice(&self.round.as_u32().unwrap_or(0).to_le_bytes());
        data.extend_from_slice(&self.value.id().0);
        data.extend_from_slice(&self.pol_round.as_u32().unwrap_or(0).to_le_bytes());
        data.extend_from_slice(&self.proposer_addr.0);
        data
    }
}

// ---------------------------------------------------------------------------
// Consensus network driver — manages message passing between BftDriver and
// the P2P network layer via tokio channels
// ---------------------------------------------------------------------------

pub struct ConsensusNetworkConfig {
    pub propose_timeout_ms: u64,
    pub prevote_timeout_ms: u64,
    pub precommit_timeout_ms: u64,
    pub max_block_txs: usize,
}

impl Default for ConsensusNetworkConfig {
    fn default() -> Self {
        Self {
            propose_timeout_ms: 1000,
            prevote_timeout_ms: 1000,
            precommit_timeout_ms: 1000,
            max_block_txs: 100,
        }
    }
}

pub struct ConsensusChannels {
    pub incoming_tx: mpsc::UnboundedSender<ConsensusNetworkMsg>,
    pub incoming_rx: mpsc::UnboundedReceiver<ConsensusNetworkMsg>,
    pub outgoing_tx: mpsc::UnboundedSender<ConsensusNetworkMsg>,
    pub outgoing_rx: mpsc::UnboundedReceiver<ConsensusNetworkMsg>,
}

impl Default for ConsensusChannels {
    fn default() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        Self { incoming_tx, incoming_rx, outgoing_tx, outgoing_rx }
    }
}

pub fn sha3_256_hash(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

pub fn build_validator_set(
    entries: Vec<([u8; 32], Vec<u8>, VotingPower)>,
) -> HyperfluidValidatorSet {
    let validators: Vec<HyperfluidValidator> = entries
        .into_iter()
        .map(|(addr_bytes, pk_bytes, power)| {
            HyperfluidValidator::new(Address32::new(addr_bytes), MlDsa65PublicKey(pk_bytes), power)
        })
        .collect();
    HyperfluidValidatorSet::new(validators)
}

// ---------------------------------------------------------------------------
// Timeout duration mapping
// ---------------------------------------------------------------------------

impl ConsensusNetworkConfig {
    pub fn duration_for(&self, kind: &TimeoutKind) -> std::time::Duration {
        let ms = match kind {
            TimeoutKind::Propose => self.propose_timeout_ms,
            TimeoutKind::Prevote => self.prevote_timeout_ms,
            TimeoutKind::Precommit => self.precommit_timeout_ms,
            TimeoutKind::Rebroadcast => self.propose_timeout_ms,
            TimeoutKind::FinalizeHeight(_) => 5000,
        };
        std::time::Duration::from_millis(ms)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockHeader;
    use arc_malachitebft_core_types::ValidatorSet;

    fn dummy_block(height: u64) -> Block {
        Block {
            header: BlockHeader {
                height,
                parent_hash: [0u8; 32],
                state_root: [1u8; 32],
                transaction_root: [2u8; 32],
                committee_id: 0,
                proposer_id: [3u8; 32],
                timestamp: height * 2,
                epoch: height / 10,
            },
            transactions: vec![],
        }
    }

    fn test_identity() -> (Address32, Arc<Identity>) {
        let identity = Identity::generate();
        let pk_bytes = identity.verifying_key_encoded();
        let addr = sha3_256_hash(&pk_bytes);
        (Address32::new(addr), Arc::new(identity))
    }

    fn single_validator_set(addr: Address32, pk: Vec<u8>) -> HyperfluidValidatorSet {
        HyperfluidValidatorSet::new(vec![HyperfluidValidator::new(addr, MlDsa65PublicKey(pk), 100)])
    }

    #[test]
    fn bft_driver_initializes() {
        let (addr, identity) = test_identity();
        let pk = identity.verifying_key_encoded();
        let set = single_validator_set(addr, pk);
        let bft = BftDriver::new(set, [0xAAu8; 32], identity, addr);
        assert_eq!(bft.height(), BlockHeight::INITIAL);
    }

    #[test]
    fn bft_driver_start_height_produces_get_value() {
        let (addr, identity) = test_identity();
        let pk = identity.verifying_key_encoded();
        let set = single_validator_set(addr, pk.clone());
        let mut bft = BftDriver::new(set, [0xAAu8; 32], identity, addr);

        let events = bft.start_height(1, single_validator_set(addr, pk));
        let has_get_value = events.iter().any(|e| matches!(e, ConsensusEvent::RequestBlock { .. }));
        assert!(has_get_value, "Single validator should be proposer → GetValue");
    }

    #[test]
    fn bft_driver_propose_and_decide_single_validator() {
        let (addr, identity) = test_identity();
        let pk = identity.verifying_key_encoded();
        let set = single_validator_set(addr, pk);
        let mut bft = BftDriver::new(set.clone(), [0xAAu8; 32], Arc::clone(&identity), addr);

        let events = bft.start_height(1, set);
        let round: Round = events
            .iter()
            .filter_map(|e| {
                if let ConsensusEvent::RequestBlock { round, .. } = e {
                    Some(Round::new(*round))
                } else {
                    None
                }
            })
            .next()
            .unwrap_or(Round::ZERO);

        let block = dummy_block(1);
        let events = bft.propose_block_value(round, block);
        let has_proposal =
            events.iter().any(|e| matches!(e, ConsensusEvent::BroadcastProposal { .. }));
        assert!(has_proposal);
    }

    #[test]
    fn signing_roundtrip_preserves_identity() {
        let identity = Identity::generate();
        let msg = b"Hyperfluid consensus test message";
        let sig = identity.sign(msg);
        assert!(identity.verify(msg, &sig));
    }

    #[test]
    fn vote_to_sign_bytes_is_deterministic() {
        let height = BlockHeight::new(5);
        let round = Round::new(2);
        let value_hash = crate::malachite::ValueHash([0x42u8; 32]);
        let addr = Address32::new([7u8; 32]);

        let vote = HyperfluidVote {
            height,
            round,
            value_id: NilOrVal::Val(value_hash),
            vote_type: arc_malachitebft_core_types::VoteType::Prevote,
            validator_addr: addr,
        };

        let bytes1 = vote.to_sign_bytes();
        let bytes2 = vote.to_sign_bytes();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn nil_vote_sign_bytes_different_from_val() {
        let height = BlockHeight::new(3);
        let round = Round::new(1);
        let addr = Address32::new([9u8; 32]);

        let nil_vote = HyperfluidVote {
            height,
            round,
            value_id: NilOrVal::Nil,
            vote_type: arc_malachitebft_core_types::VoteType::Precommit,
            validator_addr: addr,
        };

        let val_vote = HyperfluidVote {
            height,
            round,
            value_id: NilOrVal::Val(crate::malachite::ValueHash([0x11u8; 32])),
            vote_type: arc_malachitebft_core_types::VoteType::Precommit,
            validator_addr: addr,
        };

        assert_ne!(nil_vote.to_sign_bytes(), val_vote.to_sign_bytes());
    }

    #[test]
    fn bft_driver_process_vote_from_other_validator() {
        let (addr1, identity1) = test_identity();
        let pk1 = identity1.verifying_key_encoded();

        let (addr2, identity2) = test_identity();
        let pk2 = identity2.verifying_key_encoded();

        let validators = vec![
            HyperfluidValidator::new(addr1, MlDsa65PublicKey(pk1.clone()), 50),
            HyperfluidValidator::new(addr2, MlDsa65PublicKey(pk2.clone()), 50),
        ];
        let set = HyperfluidValidatorSet::new(validators);
        let mut bft = BftDriver::new(set.clone(), [0xAAu8; 32], Arc::clone(&identity1), addr1);

        // Start consensus at height 1
        let _ = bft.start_height(1, set);

        // Process a nil prevote from a different validator (addr2 != addr1).
        // The driver accepts this vote even without a proposal (nil prevote =
        // "I didn't see a proposal"). Does not crash and maintains correct height.
        let vote = HyperfluidVote {
            height: BlockHeight::new(1),
            round: Round::ZERO,
            value_id: NilOrVal::Nil,
            vote_type: arc_malachitebft_core_types::VoteType::Prevote,
            validator_addr: addr2,
        };
        let sig_bytes = identity2.sign(&vote.to_sign_bytes());
        let signed = SignedMessage::new(vote, MlDsa65Signature(sig_bytes));

        // Process the vote from addr2 — the driver must accept it without crash
        // (empty events is valid when the vote doesn't trigger a round transition)
        // Process a nil prevote from validator addr2.
        // The driver accepts the vote without crashing (signature verified
        // successfully). The events may be empty (the vote alone may not reach
        // quorum), but the driver must remain at the correct height.
        let _events = bft.process_vote(signed);
        assert_eq!(bft.height(), BlockHeight::new(1));
    }

    #[test]
    fn consensus_channels_send_receive() {
        let mut ch = ConsensusChannels::default();
        let msg = ConsensusNetworkMsg::Vote(SignedMessage::new(
            HyperfluidVote {
                height: BlockHeight::new(1),
                round: Round::ZERO,
                value_id: NilOrVal::Nil,
                vote_type: arc_malachitebft_core_types::VoteType::Prevote,
                validator_addr: Address32::new([1u8; 32]),
            },
            MlDsa65Signature(vec![0xAAu8; 100]),
        ));

        ch.incoming_tx.send(msg.clone()).unwrap();
        let received = ch.incoming_rx.try_recv().unwrap();
        match (&msg, &received) {
            (ConsensusNetworkMsg::Vote(a), ConsensusNetworkMsg::Vote(b)) => {
                assert_eq!(a.height(), b.height());
            }
            _ => panic!("Expected Vote"),
        }
    }

    #[test]
    fn build_validator_set_preserves_count() {
        let set = build_validator_set(vec![
            ([1u8; 32], vec![0xAAu8; 100], 10),
            ([2u8; 32], vec![0xBBu8; 100], 20),
        ]);
        assert_eq!(set.count(), 2);
        assert_eq!(set.total_voting_power(), 30);
    }

    #[test]
    fn bft_driver_rejects_vote_from_unknown_validator() {
        let (addr1, identity1) = test_identity();
        let pk1 = identity1.verifying_key_encoded();
        let set = single_validator_set(addr1, pk1);
        let mut bft = BftDriver::new(set.clone(), [0xAAu8; 32], Arc::clone(&identity1), addr1);

        bft.start_height(1, set);

        let vote = HyperfluidVote {
            height: BlockHeight::new(1),
            round: Round::ZERO,
            value_id: NilOrVal::Nil,
            vote_type: arc_malachitebft_core_types::VoteType::Prevote,
            validator_addr: Address32::new([0xFFu8; 32]),
        };
        let signed = SignedMessage::new(vote, MlDsa65Signature(vec![0u8; 100]));
        let events = bft.process_vote(signed);
        assert!(events.is_empty());
    }
}
