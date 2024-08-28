use super::validate::KakarotTransactionValidator;
use reth_transaction_pool::{
    blobstore::NoopBlobStore, CoinbaseTipOrdering, EthPooledTransaction, Pool, TransactionPool,
};use crate::pool::EthClient;
use serde_json::Value;
use starknet::core::types::Felt;
use std::{collections::HashSet, fs::File, io::Read, time::Duration};
use tokio::runtime::Handle;

/// A type alias for the Kakarot Transaction Validator.
/// Uses the Reth implementation [`TransactionValidationTaskExecutor`].
pub type Validator<Client> = KakarotTransactionValidator<Client, EthPooledTransaction>;

/// A type alias for the Kakarot Transaction Ordering.
/// Uses the Reth implementation [`CoinbaseTipOrdering`].
pub type TransactionOrdering = CoinbaseTipOrdering<EthPooledTransaction>;

/// A type alias for the Kakarot Sequencer Mempool.
pub type KakarotPool<Client> = Pool<Validator<Client>, TransactionOrdering, NoopBlobStore>;

#[derive(Debug, Default)]
pub struct AccountManager {
    accounts: HashSet<Felt>,
}

impl AccountManager {
    pub fn new(path: &str) -> Self {
        let mut accounts = HashSet::new();

        // Open the file specified by `path`
        let Ok(mut file) = File::open(path) else {
            return Self::default();
        };

        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_err() {
            return Self::default();
        }

        // Parse the file contents as JSON
        let json: Value = match serde_json::from_str(&contents) {
            Ok(json) => json,
            Err(_) => {
                return Self::default();
            }
        };

        // Extract the account addresses from the JSON array
        if let Some(array) = json.as_array() {
            for item in array {
                if let Some(address) = item.as_str() {
                    accounts.insert(Felt::from_hex_unchecked(address));
                }
            }
        }

        Self { accounts }
    }

    pub fn start<SP>(&self, rt_handle: &Handle, eth_client: &'static EthClient<SP>)
    where
        SP: starknet::providers::Provider + Send + Sync+Clone + 'static,
    {
        let accounts = self.accounts.clone();

        rt_handle.spawn(async move {
            loop {
                for address in &accounts {
                    Self::process_transaction(address, eth_client);
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    fn process_transaction<SP>(address: &Felt, eth_client: &EthClient<SP>)
    where
        SP: starknet::providers::Provider + Send + Sync+Clone + 'static,
    {
        let balance = Self::check_balance(address);

        if balance > Felt::ONE {
            let best_hashes =
                eth_client.mempool().as_ref().best_transactions().map(|x| *x.hash()).collect::<Vec<_>>();

            if let Some(best_hash) = best_hashes.first() {
                eth_client.mempool().as_ref().remove_transactions(vec![*best_hash]);
            }
        }
    }

    const fn check_balance(_address: &Felt) -> Felt {
        Felt::ONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_manager_new() {
        let account_manager = AccountManager::new("src/pool/accounts.json");

        let accounts = account_manager.accounts;

        assert!(accounts
            .contains(&Felt::from_hex_unchecked("0x00686735619287df0f11ec4cda22675f780886b52bf59cf899dd57fd5d5f4cad")));
        assert!(accounts
            .contains(&Felt::from_hex_unchecked("0x0332825a42ccbec3e2ceb6c242f4dff4682e7d16b8559104b5df8fd925ddda09")));
        assert!(accounts
            .contains(&Felt::from_hex_unchecked("0x003f5628053c2d6bdfc9e45ea8aeb14405b8917226d455a94b3225a9a7520559")));
    }
}
