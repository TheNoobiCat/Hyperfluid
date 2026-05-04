# Conformance Test Index — Stage 00 (Foundation)

| Spec Section | Test ID | Description | Status | Location |
|-------------|---------|-------------|--------|----------|
| consensus-spec.md S1.3 | CT-CONSENSUS-001 | BlockHeader serde roundtrip | PASS | `hyperfluid-consensus::types::tests::block_header_is_copyable_after_serde` |
| consensus-spec.md S1.3 | CT-CONSENSUS-002 | TxType has all 11 variants | PASS | `hyperfluid-consensus::types::tests::tx_type_variants_are_exhaustive` |
| consensus-spec.md S1.3 | CT-CONSENSUS-003 | Committee size = 100 validation | PASS | `hyperfluid-consensus::types::tests::committee_size_validation` |
| consensus-spec.md S1.3 | CT-CONSENSUS-004 | GenesisConfig serde roundtrip | PASS | `hyperfluid-consensus::genesis::tests::genesis_config_serde_roundtrip` |
| consensus-spec.md S1.3 | CT-CONSENSUS-005 | Testnet genesis has airdrop agent with full supply | PASS | `hyperfluid-consensus::genesis::tests::testnet_genesis_has_airdrop_agent` |
| consensus-spec.md S1.3 | CT-CONSENSUS-006 | Testnet genesis has single validator at min stake | PASS | `hyperfluid-consensus::genesis::tests::testnet_genesis_has_single_validator` |
| consensus-spec.md S1.3 | CT-CONSENSUS-007 | System params match spec defaults | PASS | `hyperfluid-consensus::genesis::tests::system_params_match_spec_defaults` |
| consensus-spec.md S2.3 | CT-STATE-001 | KeyPrefix 0x01-0x0F roundtrip | PASS | `hyperfluid-state::tests::key_prefix_roundtrip` |
| consensus-spec.md S2.3 | CT-STATE-002 | Invalid KeyPrefix bytes rejected | PASS | `hyperfluid-state::tests::key_prefix_invalid_byte` |
| consensus-spec.md S2.2 | CT-STATE-003 | State key deterministic from same inputs | PASS | `hyperfluid-state::tests::state_key_is_deterministic` |
| consensus-spec.md S2.2 | CT-STATE-004 | Different prefix = different key | PASS | `hyperfluid-state::tests::state_key_different_prefix_different_key` |
| consensus-spec.md S2.2 | CT-STATE-005 | Different ID = different key | PASS | `hyperfluid-state::tests::state_key_different_id_different_key` |
| consensus-spec.md S2.3 | CT-STATE-006 | SHA3-256 is deterministic | PASS | `hyperfluid-state::tests::sha3_256_is_deterministic` |
| consensus-spec.md S2.3 | CT-STATE-007 | Account pubkey revealed on first spend matches hash | PASS | `hyperfluid-state::tests::account_reveals_pubkey_on_first_spend` |
| staking-spec.md S1.3 | CT-STAKING-001 | ValidatorState has exactly 4 states | PASS | `hyperfluid-staking::tests::validator_state_has_four_variants` |
| staking-spec.md S1.3 | CT-STAKING-002 | ValidatorRecord serde roundtrip | PASS | `hyperfluid-staking::tests::validator_record_serde_roundtrip` |
| staking-spec.md S1.3 | CT-STAKING-003 | SystemParameters defaults match spec values | PASS | `hyperfluid-staking::tests::system_parameters_defaults_match_spec` |
| staking-spec.md S2.3 | CT-STAKING-004 | GovernanceVoteTx serde roundtrip | PASS | `hyperfluid-staking::tests::governance_vote_tx_roundtrip` |
| consensus-spec.md S1.3 | CT-NODE-001 | Genesis block is height 0 with zero parent hash | PASS | `hyperfluid-node::tests::genesis_block_height_zero` |
| consensus-spec.md S1.3 | CT-NODE-002 | Genesis block is epoch 0 | PASS | `hyperfluid-node::tests::genesis_block_epoch_zero` |
| consensus-spec.md S1.3 | CT-NODE-003 | Genesis block timestamp matches config | PASS | `hyperfluid-node::tests::genesis_block_timestamp_matches_config` |

**Total:** 21 conformance tests, 21 PASS, 0 FAIL
