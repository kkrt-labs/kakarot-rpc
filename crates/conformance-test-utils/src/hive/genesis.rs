use std::collections::HashMap;

use ethers::types::{Address, Bytes, H256, I256, U256, U64};
use serde::{Deserialize, Serialize};

/// Types from https://github.com/ethereum/go-ethereum/blob/master/core/genesis.go#L49C1-L58
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct HiveGenesisConfig {
    pub config: Config,
    pub coinbase: Address,
    pub difficulty: I256,
    pub extraData: Bytes,
    pub gasLimit: U64,
    pub nonce: U64,
    pub timestamp: U64,
    pub alloc: HashMap<Address, AccountInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Config {
    pub chainId: i128,
    pub homesteadBlock: i128,
    pub eip150Block: i128,
    pub eip150Hash: H256,
    pub eip155Block: i128,
    pub eip158Block: i128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountInfo {
    pub balance: U256,
    pub code: Option<Bytes>,
    pub storage: Option<HashMap<U256, U256>>,
}

#[cfg(test)]
mod tests {
    use ethers::types::U256;

    use super::HiveGenesisConfig;

    #[test]
    fn test_read_hive_genesis() {
        // Read the hive genesis file
        let genesis: HiveGenesisConfig = serde_json::from_str(std::include_str!("./genesis.json")).unwrap();

        // Verify the genesis file has the expected number of accounts
        assert_eq!(genesis.alloc.len(), 7);

        // Verify balance of each account is not empty
        assert!(genesis.alloc.values().all(|account_info| account_info.balance >= U256::from(0)));

        // Verify the storage field for each account
        // Since there is only one account with non-empty storage, we can hardcode the expected values
        assert!(genesis.alloc.values().all(|account_info| {
            account_info.storage.as_ref().map_or(true, |storage| {
                storage.len() == 2
                    && storage.get(&U256::from(0)) == Some(&U256::from("0x1234"))
                    && storage.get(&U256::from("0x6661e9d6d8b923d5bbaab1b96e1dd51ff6ea2a93520fdc9eb75d059238b8c5e9"))
                        == Some(&U256::from("0x01"))
            })
        }));

        // Verify the code field for each account, if exists, is not empty
        assert!(
            genesis.alloc.values().all(|account_info| account_info.code.as_ref().map_or(true, |code| !code.is_empty()))
        );
    }
}
