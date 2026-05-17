// === Malachite BFT Integration ===
//
// Integrates Malachite core-* crates as pure libraries per ADR-0018.
// Implements SigningScheme for ML-DSA-65 and Context for Hyperfluid types.
//
// Remaining work (ADR-0018):
//   - Effect handler: route Malachite Effects to clatter network + tokio timers (~300 lines)
//   - Clatter network bridge: send/receive consensus messages over PQ-Noise (~500 lines)
//   - Host actor: proposal building, block validation, vote extensions, commit (~400 lines)
//
// Source: ADR-0018, docs/specs/protocol/consensus-spec.md

use std::fmt;
use std::vec::Vec;

use arc_malachitebft_core_types::{
    Address, Context, Extension, Height, NilOrVal, Proposal, ProposalPart, Round, SigningScheme,
    Validator, ValidatorSet, Value, Vote, VoteType, VotingPower,
};
use ml_dsa::{MlDsa65, SigningKey};

use crate::types::Block;

// ---------------------------------------------------------------------------
// Address32 — wraps [u8;32] with Display for Malachite Address trait
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address32(pub [u8; 32]);

impl Address32 {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for Address32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Address for Address32 {}

// ---------------------------------------------------------------------------
// ML-DSA-65 SigningScheme
//
// Uses byte-level wrappers because ml_dsa types (Signature, VerifyingKey)
// do not implement Ord/Eq required by Malachite trait bounds.
// ---------------------------------------------------------------------------

/// Signature backed by raw bytes for deterministic Ord/Eq.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MlDsa65Signature(pub Vec<u8>);

/// Public key backed by raw bytes for deterministic Eq.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MlDsa65PublicKey(pub Vec<u8>);

#[derive(Clone)]
pub struct MlDsa65PrivateKey(pub SigningKey<MlDsa65>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MlDsa65Scheme;

#[derive(Debug)]
pub struct MlDsa65DecodingError(String);

impl fmt::Display for MlDsa65DecodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ML-DSA-65 signature decoding error: {}", self.0)
    }
}

impl SigningScheme for MlDsa65Scheme {
    type DecodingError = MlDsa65DecodingError;
    type Signature = MlDsa65Signature;
    type PublicKey = MlDsa65PublicKey;
    type PrivateKey = MlDsa65PrivateKey;

    fn decode_signature(bytes: &[u8]) -> Result<Self::Signature, Self::DecodingError> {
        use core::convert::TryFrom;
        // Validate that bytes form a valid ML-DSA-65 signature
        ml_dsa::Signature::<MlDsa65>::try_from(bytes)
            .map_err(|e| MlDsa65DecodingError(format!("{}", e)))?;
        Ok(MlDsa65Signature(bytes.to_vec()))
    }

    fn encode_signature(signature: &Self::Signature) -> Vec<u8> {
        signature.0.clone()
    }
}

// ---------------------------------------------------------------------------
// BlockHeight — implements Height
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BlockHeight(u64);

impl BlockHeight {
    pub fn new(h: u64) -> Self {
        Self(h)
    }
}

impl fmt::Display for BlockHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Height for BlockHeight {
    const ZERO: Self = Self(0);
    const INITIAL: Self = Self(1);

    fn increment_by(&self, n: u64) -> Self {
        Self(self.0.saturating_add(n))
    }

    fn decrement_by(&self, n: u64) -> Option<Self> {
        self.0.checked_sub(n).map(Self)
    }

    fn as_u64(&self) -> u64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// ValueHash + BlockValue — implements Value
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueHash(pub [u8; 32]);

impl fmt::Display for ValueHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Consensus value wrapping a Hyperfluid Block.
/// Ord is derived from the block hash (ValueHash), ensuring deterministic ordering.
#[derive(Clone, Debug)]
pub struct BlockValue {
    pub block: Block,
    hash: ValueHash,
}

impl BlockValue {
    pub fn new(block: Block) -> Self {
        let hash = ValueHash(block.header.block_hash());
        Self { block, hash }
    }
}

impl PartialEq for BlockValue {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for BlockValue {}

impl PartialOrd for BlockValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BlockValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl Value for BlockValue {
    type Id = ValueHash;

    fn id(&self) -> Self::Id {
        self.hash
    }
}

// ---------------------------------------------------------------------------
// HyperfluidValidator — implements Validator
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperfluidValidator {
    pub addr: Address32,
    pub pubkey: MlDsa65PublicKey,
    pub voting_power: VotingPower,
}

impl HyperfluidValidator {
    pub fn new(addr: Address32, pubkey: MlDsa65PublicKey, voting_power: VotingPower) -> Self {
        Self { addr, pubkey, voting_power }
    }
}

impl Validator<HyperfluidContext> for HyperfluidValidator {
    fn address(&self) -> &Address32 {
        &self.addr
    }

    fn public_key(&self) -> &MlDsa65PublicKey {
        &self.pubkey
    }

    fn voting_power(&self) -> VotingPower {
        self.voting_power
    }
}

// ---------------------------------------------------------------------------
// HyperfluidValidatorSet — implements ValidatorSet
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperfluidValidatorSet {
    validators: Vec<HyperfluidValidator>,
    total_power: VotingPower,
}

impl HyperfluidValidatorSet {
    pub fn new(mut validators: Vec<HyperfluidValidator>) -> Self {
        validators
            .sort_by(|a, b| b.voting_power.cmp(&a.voting_power).then_with(|| a.addr.cmp(&b.addr)));
        let total = validators.iter().map(|v| v.voting_power).sum();
        Self { validators, total_power: total }
    }
}

impl ValidatorSet<HyperfluidContext> for HyperfluidValidatorSet {
    fn count(&self) -> usize {
        self.validators.len()
    }

    fn total_voting_power(&self) -> VotingPower {
        self.total_power
    }

    fn get_by_address(&self, address: &Address32) -> Option<&HyperfluidValidator> {
        self.validators.iter().find(|v| v.addr == *address)
    }

    fn get_by_index(&self, index: usize) -> Option<&HyperfluidValidator> {
        self.validators.get(index)
    }
}

// ---------------------------------------------------------------------------
// HyperfluidVote — implements Vote
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperfluidVote {
    pub height: BlockHeight,
    pub round: Round,
    pub value_id: NilOrVal<ValueHash>,
    pub vote_type: VoteType,
    pub validator_addr: Address32,
}

impl Vote<HyperfluidContext> for HyperfluidVote {
    fn height(&self) -> BlockHeight {
        self.height
    }

    fn round(&self) -> Round {
        self.round
    }

    fn value(&self) -> &NilOrVal<ValueHash> {
        &self.value_id
    }

    fn take_value(self) -> NilOrVal<ValueHash> {
        self.value_id
    }

    fn vote_type(&self) -> VoteType {
        self.vote_type
    }

    fn validator_address(&self) -> &Address32 {
        &self.validator_addr
    }

    fn extension(
        &self,
    ) -> Option<&arc_malachitebft_core_types::SignedExtension<HyperfluidContext>> {
        None
    }

    fn take_extension(
        &mut self,
    ) -> Option<arc_malachitebft_core_types::SignedExtension<HyperfluidContext>> {
        None
    }

    fn extend(
        self,
        _extension: arc_malachitebft_core_types::SignedExtension<HyperfluidContext>,
    ) -> Self {
        self
    }
}

// ---------------------------------------------------------------------------
// HyperfluidProposal — implements Proposal
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperfluidProposal {
    pub height: BlockHeight,
    pub round: Round,
    pub value: BlockValue,
    pub pol_round: Round,
    pub proposer_addr: Address32,
}

impl Proposal<HyperfluidContext> for HyperfluidProposal {
    fn height(&self) -> BlockHeight {
        self.height
    }

    fn round(&self) -> Round {
        self.round
    }

    fn value(&self) -> &BlockValue {
        &self.value
    }

    fn take_value(self) -> BlockValue {
        self.value
    }

    fn pol_round(&self) -> Round {
        self.pol_round
    }

    fn validator_address(&self) -> &Address32 {
        &self.proposer_addr
    }
}

// ---------------------------------------------------------------------------
// HyperfluidProposalPart — implements ProposalPart
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperfluidProposalPart {
    pub first: bool,
    pub last: bool,
}

impl ProposalPart<HyperfluidContext> for HyperfluidProposalPart {
    fn is_first(&self) -> bool {
        self.first
    }

    fn is_last(&self) -> bool {
        self.last
    }
}

// ---------------------------------------------------------------------------
// HyperfluidExtension — implements Extension
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperfluidExtension;

impl Extension for HyperfluidExtension {
    fn size_bytes(&self) -> usize {
        0
    }
}

// ---------------------------------------------------------------------------
// HyperfluidContext — implements Context
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct HyperfluidContext {
    pub validator_set: HyperfluidValidatorSet,
    pub proposer_seed: [u8; 32],
}

impl HyperfluidContext {
    pub fn new(validator_set: HyperfluidValidatorSet, proposer_seed: [u8; 32]) -> Self {
        Self { validator_set, proposer_seed }
    }
}

impl Context for HyperfluidContext {
    type Address = Address32;
    type Height = BlockHeight;
    type ProposalPart = HyperfluidProposalPart;
    type Proposal = HyperfluidProposal;
    type Validator = HyperfluidValidator;
    type ValidatorSet = HyperfluidValidatorSet;
    type Timeouts = arc_malachitebft_core_types::LinearTimeouts;
    type Value = BlockValue;
    type Vote = HyperfluidVote;
    type Extension = HyperfluidExtension;
    type SigningScheme = MlDsa65Scheme;

    fn select_proposer<'a>(
        &self,
        validator_set: &'a Self::ValidatorSet,
        height: Self::Height,
        round: Round,
    ) -> &'a Self::Validator {
        let count = validator_set.count();
        assert!(count > 0, "select_proposer called on empty validator set");

        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(height.as_u64().to_le_bytes());
        hasher.update(round.as_u32().unwrap_or(0).to_le_bytes());
        hasher.update(self.proposer_seed);
        let hash = hasher.finalize();
        let selector = u64::from_le_bytes(hash[..8].try_into().unwrap());
        let index = (selector as usize) % count;
        validator_set.get_by_index(index).expect("validator at index")
    }

    fn new_proposal(
        &self,
        height: Self::Height,
        round: Round,
        value: Self::Value,
        pol_round: Round,
        address: Self::Address,
    ) -> Self::Proposal {
        HyperfluidProposal { height, round, value, pol_round, proposer_addr: address }
    }

    fn new_prevote(
        &self,
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<ValueHash>,
        address: Self::Address,
    ) -> Self::Vote {
        HyperfluidVote {
            height,
            round,
            value_id,
            vote_type: VoteType::Prevote,
            validator_addr: address,
        }
    }

    fn new_precommit(
        &self,
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<ValueHash>,
        address: Self::Address,
    ) -> Self::Vote {
        HyperfluidVote {
            height,
            round,
            value_id,
            vote_type: VoteType::Precommit,
            validator_addr: address,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockHeader;

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

    fn test_validator(id: u8, power: u64) -> HyperfluidValidator {
        use ml_dsa::{Generate, KeyExport, Keypair};
        let keypair = SigningKey::<MlDsa65>::generate();
        let pk_bytes = keypair.verifying_key().to_bytes().to_vec();
        let pubkey = MlDsa65PublicKey(pk_bytes);
        let mut addr = [0u8; 32];
        addr[0] = id;
        HyperfluidValidator::new(Address32::new(addr), pubkey, power)
    }

    // ── SigningScheme ──

    #[test]
    fn signing_scheme_encode_decode_roundtrip() {
        use ml_dsa::{Generate, SignatureEncoding, Signer};
        let keypair = SigningKey::<MlDsa65>::generate();
        let message = b"Hyperfluid consensus vote";
        let sig = keypair.sign(message);
        let sig_bytes = sig.to_vec();

        // Roundtrip through our byte wrapper
        let encoded = MlDsa65Scheme::encode_signature(&MlDsa65Signature(sig_bytes.clone()));
        assert_eq!(encoded, sig_bytes);
        let decoded = MlDsa65Scheme::decode_signature(&encoded).expect("decode roundtrip");
        assert_eq!(MlDsa65Signature(sig_bytes), decoded);
    }

    #[test]
    fn signing_scheme_decode_invalid_bytes() {
        let result = MlDsa65Scheme::decode_signature(b"too short");
        assert!(result.is_err());
    }

    // ── BlockHeight ──

    #[test]
    fn block_height_trait_methods() {
        let h = BlockHeight::new(10);
        assert_eq!(h.as_u64(), 10);
        assert_eq!(h.increment_by(5), BlockHeight::new(15));
        assert_eq!(h.decrement_by(3), Some(BlockHeight::new(7)));
    }

    #[test]
    fn block_height_constants() {
        assert_eq!(BlockHeight::ZERO, BlockHeight::new(0));
        assert_eq!(BlockHeight::INITIAL, BlockHeight::new(1));
    }

    // ── BlockValue ──

    #[test]
    fn block_value_id_is_block_hash() {
        let block = dummy_block(42);
        let value = BlockValue::new(block.clone());
        assert_eq!(value.id(), ValueHash(block.header.block_hash()));
    }

    #[test]
    fn block_value_ord_by_hash() {
        let a = BlockValue::new(dummy_block(1));
        let b = BlockValue::new(dummy_block(1));
        assert_eq!(a, b);
        assert!(a <= b);

        let c = BlockValue::new(dummy_block(2));
        assert_ne!(a, c);
    }

    // ── ValidatorSet ──

    #[test]
    fn validator_set_sorted_by_power_then_address() {
        let a = test_validator(10, 50);
        let b = test_validator(20, 100);
        let c = test_validator(30, 50);

        let set = HyperfluidValidatorSet::new(vec![a.clone(), b.clone(), c.clone()]);
        assert_eq!(set.count(), 3);
        assert_eq!(set.total_voting_power(), 200);

        let ordered: Vec<Address32> =
            (0..set.count()).map(|i| *set.get_by_index(i).unwrap().address()).collect();
        assert_eq!(ordered[0], b.addr);
        assert_eq!(ordered[1], a.addr);
        assert_eq!(ordered[2], c.addr);
    }

    #[test]
    fn validator_set_lookup() {
        let v = test_validator(77, 100);
        let set = HyperfluidValidatorSet::new(vec![v.clone()]);
        let mut addr77 = [0u8; 32];
        addr77[0] = 77;
        assert_eq!(set.get_by_address(&Address32::new(addr77)).unwrap(), &v);
        assert!(set.get_by_address(&Address32::new([99u8; 32])).is_none());
    }

    // ── Context ──

    #[test]
    fn context_select_proposer_deterministic() {
        let validators: Vec<_> = (0..10).map(|i| test_validator(i, 100 - i as u64 * 10)).collect();
        let set = HyperfluidValidatorSet::new(validators);
        let ctx = HyperfluidContext::new(set, [0xAAu8; 32]);

        let p1 = ctx.select_proposer(&ctx.validator_set, BlockHeight::new(1), Round::ZERO);
        let p2 = ctx.select_proposer(&ctx.validator_set, BlockHeight::new(1), Round::ZERO);
        assert_eq!(p1.address(), p2.address());
    }

    #[test]
    fn context_create_proposal() {
        let set = HyperfluidValidatorSet::new(vec![test_validator(1, 100)]);
        let ctx = HyperfluidContext::new(set, [0u8; 32]);
        let value = BlockValue::new(dummy_block(1));

        let proposal = ctx.new_proposal(
            BlockHeight::new(1),
            Round::ZERO,
            value.clone(),
            Round::Nil,
            Address32::new([1u8; 32]),
        );
        assert_eq!(proposal.height(), BlockHeight::new(1));
        assert_eq!(proposal.round(), Round::ZERO);
        assert_eq!(proposal.value(), &value);
        assert_eq!(proposal.pol_round(), Round::Nil);
    }

    #[test]
    fn context_create_prevote() {
        let set = HyperfluidValidatorSet::new(vec![test_validator(1, 100)]);
        let ctx = HyperfluidContext::new(set, [0u8; 32]);
        let value_id = BlockValue::new(dummy_block(1)).id();

        let vote = ctx.new_prevote(
            BlockHeight::new(1),
            Round::ZERO,
            NilOrVal::Val(value_id),
            Address32::new([1u8; 32]),
        );
        assert_eq!(vote.vote_type(), VoteType::Prevote);
        assert_eq!(*vote.value(), NilOrVal::Val(value_id));
    }

    #[test]
    fn context_create_precommit_nil() {
        let set = HyperfluidValidatorSet::new(vec![test_validator(1, 100)]);
        let ctx = HyperfluidContext::new(set, [0u8; 32]);

        let vote = ctx.new_precommit(
            BlockHeight::new(3),
            Round::new(2),
            NilOrVal::Nil,
            Address32::new([1u8; 32]),
        );
        assert_eq!(vote.vote_type(), VoteType::Precommit);
        assert_eq!(vote.height(), BlockHeight::new(3));
        assert_eq!(vote.round(), Round::new(2));
        assert!(vote.value().is_nil());
    }

    #[test]
    fn full_proposal_vote_cycle() {
        let validators: Vec<_> = (1..=4).map(|i| test_validator(i, 250)).collect();
        let set = HyperfluidValidatorSet::new(validators);
        let ctx = HyperfluidContext::new(set.clone(), [0x42u8; 32]);

        let height = BlockHeight::new(5);
        let value = BlockValue::new(dummy_block(5));
        let value_id = value.id();

        let proposer = ctx.select_proposer(&ctx.validator_set, height, Round::ZERO);
        let addr = *proposer.address();

        let proposal = ctx.new_proposal(height, Round::ZERO, value, Round::Nil, addr);
        assert_eq!(*proposal.validator_address(), addr);

        let prevote = ctx.new_prevote(height, Round::ZERO, NilOrVal::Val(value_id), addr);
        assert_eq!(prevote.vote_type(), VoteType::Prevote);

        let precommit = ctx.new_precommit(height, Round::new(1), NilOrVal::Val(value_id), addr);
        assert_eq!(precommit.vote_type(), VoteType::Precommit);
        assert_eq!(precommit.height(), height);

        // Verify all validators are accessible by address
        for i in 0..set.count() {
            let v = set.get_by_index(i).unwrap();
            let lookup = set.get_by_address(v.address()).unwrap();
            assert_eq!(v.voting_power(), lookup.voting_power());
        }
    }
}
