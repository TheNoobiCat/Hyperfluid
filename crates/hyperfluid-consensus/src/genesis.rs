// === Genesis configuration ===
//
// Source: specs/protocol/consensus-spec.md Sections 1-2,
//         specs/protocol/staking-spec.md Section 1,
//         FR-0153a (Genesis-Only Mint)

use serde::{Deserialize, Serialize};

pub use crate::types::Hash32;

/// Canonical genesis block one-time allocation record.
/// An account appearing here exists in the SMT at genesis with
/// the listed balance and optional validator stake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub account_id: Hash32,
    pub balance: u128,
    pub pubkey: Option<Vec<u8>>,
}

/// A validator registered at genesis.
/// Must reference an account_id that appears in genesis_accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisValidator {
    pub validator_id: Hash32,
    pub bonded_stake: u128,
}

/// Genesis block configuration — the single source of truth
/// for the chain's initial state. This format is the production
/// format; the testnet scaffold uses the exact same structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain_id: String,
    pub timestamp: u64,
    pub epoch_length: u64,
    pub committee_size: u64,
    pub min_stake: u128,
    pub bond_delay: u64,
    pub unbond_delay: u64,
    pub max_governance_proposals: u64,
    pub proposal_deposit: u128,
    pub liveness_window_blocks: u64,
    pub liveness_miss_threshold_pct: u8,
    pub total_agx_supply: u128,
    pub airdrop_amount_per_agent: u128,
    pub accounts: Vec<GenesisAccount>,
    pub validators: Vec<GenesisValidator>,
}

impl GenesisConfig {
    pub fn new_testnet_single_validator() -> Self {
        let validator_pubkey_bytes = vec![0xABu8; 32];
        let mut account_id = [0u8; 32];
        account_id.copy_from_slice(&validator_pubkey_bytes);

        let airdrop_agent_id = [0xCDu8; 32];

        // All monetary values are in atto-AGX (10^-18 AGX).
        // 1 AGX = 1_000_000_000_000_000_000 atto-AGX (10^18)
        Self {
            chain_id: "hyperfluid-testnet-1".into(),
            timestamp: 0,
            epoch_length: 8192,
            committee_size: 100,
            min_stake: 1_000_000_000_000_000_000_000u128, // 1,000 AGX in atto-AGX
            bond_delay: 8640,
            unbond_delay: 120_960,
            max_governance_proposals: 32,
            proposal_deposit: 500_000_000_000_000_000_000u128, // 500 AGX in atto-AGX
            liveness_window_blocks: 8192,
            liveness_miss_threshold_pct: 20,
            // 10M AGX = 10^25 atto-AGX (fits in u128: max ~3.4 * 10^38)
            total_agx_supply: 10_000_000_000_000_000_000_000_000u128,
            airdrop_amount_per_agent: 100_000_000_000_000_000_000u128, // 100 AGX in atto-AGX
            accounts: vec![
                GenesisAccount {
                    account_id: airdrop_agent_id,
                    balance: 10_000_000_000_000_000_000_000_000u128, // entire supply (10M AGX)
                    pubkey: None,
                },
                GenesisAccount {
                    account_id,
                    balance: 2_000_000_000_000_000_000_000u128, // 2,000 AGX for validator + seed tasks
                    pubkey: Some(validator_pubkey_bytes.clone()),
                },
            ],
            validators: vec![GenesisValidator {
                validator_id: account_id,
                bonded_stake: 1_000_000_000_000_000_000_000u128, // 1,000 AGX (min)
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testnet_genesis_has_airdrop_agent() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let airdrop = genesis.accounts.iter().find(|a| a.account_id == [0xCDu8; 32]);
        assert!(airdrop.is_some());
        assert_eq!(airdrop.unwrap().balance, genesis.total_agx_supply);
    }

    #[test]
    fn testnet_genesis_has_single_validator() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        assert_eq!(genesis.validators.len(), 1);
        assert_eq!(genesis.validators[0].bonded_stake, genesis.min_stake);
    }

    #[test]
    fn genesis_total_supply_is_10m_agx() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        // 10,000,000 AGX in atto-AGX = 10^25
        assert_eq!(genesis.total_agx_supply, 10_000_000_000_000_000_000_000_000u128);
    }

    #[test]
    fn genesis_airdrop_amount_is_100_agx() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        // 100 AGX in atto-AGX = 10^20
        assert_eq!(genesis.airdrop_amount_per_agent, 100_000_000_000_000_000_000u128);
    }

    #[test]
    fn genesis_config_serde_roundtrip() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let json = serde_json::to_string_pretty(&genesis).unwrap();
        let genesis2: GenesisConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(genesis, genesis2);
    }

    #[test]
    fn system_params_match_spec_defaults() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        assert_eq!(genesis.epoch_length, 8192);
        assert_eq!(genesis.committee_size, 100);
        assert_eq!(genesis.min_stake, 1_000_000_000_000_000_000_000u128);
        assert_eq!(genesis.proposal_deposit, 500_000_000_000_000_000_000u128);
        assert_eq!(genesis.unbond_delay, 120_960); // 14 days in blocks
        assert_eq!(genesis.liveness_miss_threshold_pct, 20);
    }
}
