use std::collections::HashMap;

use reth_primitives::{Address, Bytes, H256, U256, U64};
use serde::{Deserialize, Serialize};

/// Types from https://github.com/ethereum/go-ethereum/blob/master/core/genesis.go#L49C1-L58
#[derive(Serialize, Deserialize, Debug)]
pub struct HiveGenesisConfig {
    pub config: Config,
    pub coinbase: Address,
    pub difficulty: U64,
    #[serde(rename = "extraData")]
    pub extra_data: Bytes,
    #[serde(rename(deserialize = "gasLimit"))]
    pub gas_limit: U64,
    pub nonce: U64,
    pub timestamp: U64,
    pub alloc: HashMap<Address, AccountInfo>,
}

impl HiveGenesisConfig {
    pub fn new() -> Result<Self, serde_json::Error> {
        serde_json::from_str(std::include_str!("./genesis.json"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(rename(deserialize = "chainId"))]
    pub chain_id: i128,
    #[serde(rename(deserialize = "homesteadBlock"))]
    pub homestead_block: i128,
    #[serde(rename(deserialize = "eip150Block"))]
    pub eip150_block: i128,
    #[serde(rename(deserialize = "eip150Hash"))]
    pub eip150_hash: H256,
    #[serde(rename(deserialize = "eip155Block"))]
    pub eip155_block: i128,
    #[serde(rename(deserialize = "eip158Block"))]
    pub eip158_block: i128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountInfo {
    pub balance: U256,
    pub code: Option<Bytes>,
    pub storage: Option<HashMap<U256, U256>>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reth_primitives::U256;

    use super::HiveGenesisConfig;

    #[test]
    fn test_read_hive_genesis() {
        // Read the hive genesis file
        let genesis = HiveGenesisConfig::new().expect("Failed to read genesis.json");

        // Verify the genesis file has the expected number of accounts
        assert_eq!(genesis.alloc.len(), 7);

        // Verify balance of each account is not empty
        assert!(genesis.alloc.values().all(|account_info| account_info.balance >= U256::from(0)));

        // Verify the storage field for each account
        // Since there is only one account with non-empty storage, we can hardcode the expected values
        assert!(genesis.alloc.values().all(|account_info| {
            account_info.storage.as_ref().map_or(true, |storage| {
                storage.len() == 2
                    && *storage
                        .get(
                            &U256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")
                                .unwrap(),
                        )
                        .unwrap()
                        == U256::from_str("0x1234").unwrap()
                    && *storage
                        .get(
                            &U256::from_str("0x6661e9d6d8b923d5bbaab1b96e1dd51ff6ea2a93520fdc9eb75d059238b8c5e9")
                                .unwrap(),
                        )
                        .unwrap()
                        == U256::from_str("0x01").unwrap()
            })
        }));

        // Verify the code field for each account, if exists, is not empty
        assert!(
            genesis.alloc.values().all(|account_info| account_info.code.as_ref().map_or(true, |code| !code.is_empty()))
        );
    }
}
