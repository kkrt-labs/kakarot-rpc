use super::validate::KakarotTransactionValidator;
use crate::client::EthClient;
use futures::future::select_all;
use reth_primitives::{BlockId, U256};
use reth_transaction_pool::{
    blobstore::NoopBlobStore, CoinbaseTipOrdering, EthPooledTransaction, Pool, TransactionPool,
};
use serde_json::Value;
use starknet::core::types::Felt;
use std::{collections::HashMap, fs::File, io::Read, str::FromStr, sync::Arc, time::Duration};
use tokio::{
    runtime::Handle,
    sync::{Mutex, MutexGuard},
};

/// A type alias for the Kakarot Transaction Validator.
/// Uses the Reth implementation [`TransactionValidationTaskExecutor`].
pub type Validator<Client> = KakarotTransactionValidator<Client, EthPooledTransaction>;

/// A type alias for the Kakarot Transaction Ordering.
/// Uses the Reth implementation [`CoinbaseTipOrdering`].
pub type TransactionOrdering = CoinbaseTipOrdering<EthPooledTransaction>;

/// A type alias for the Kakarot Sequencer Mempool.
pub type KakarotPool<Client> = Pool<Validator<Client>, TransactionOrdering, NoopBlobStore>;

/// Manages a collection of accounts and their associated nonces, interfacing with an Ethereum client.
///
/// This struct provides functionality to initialize account data from a file, monitor account balances,
/// and process transactions for accounts with sufficient balance.
#[derive(Debug)]
pub struct AccountManager<SP: starknet::providers::Provider + Send + Sync + Clone + 'static> {
    /// A shared, mutable collection of accounts and their nonces.
    accounts: HashMap<Felt, Arc<Mutex<Felt>>>,
    /// The Ethereum client used to interact with the blockchain.
    eth_client: Arc<EthClient<SP>>,
}

impl<SP: starknet::providers::Provider + Send + Sync + Clone + 'static> AccountManager<SP> {
    /// Creates a new [`AccountManager`] instance by initializing account data from a JSON file.
    pub async fn new(path: &str, eth_client: Arc<EthClient<SP>>) -> eyre::Result<Self> {
        let mut accounts = HashMap::new();

        // Open the file specified by `path`
        let mut file = File::open(path)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // Parse the file contents as JSON
        let json: Value = serde_json::from_str(&contents)?;

        // Extract the account addresses from the JSON array of objects
        if let Some(array) = json.as_array() {
            for item in array {
                if let Some(address_value) = item.get("address") {
                    if let Some(account_address) = address_value.as_str() {
                        let felt_address = Felt::from_str(account_address)
                            .map_err(|e| eyre::eyre!("Error converting account address to Felt: {:?}", e))?;

                        let starknet_block_id = eth_client
                            .eth_provider()
                            .to_starknet_block_id(Some(BlockId::default()))
                            .await
                            .map_err(|e| eyre::eyre!("Error converting block ID: {:?}", e))?;

                        // Query the initial account_nonce for the account from the provider
                        let nonce = eth_client
                            .starknet_provider()
                            .get_nonce(starknet_block_id, felt_address)
                            .await
                            .unwrap_or_default();
                        accounts.insert(felt_address, Arc::new(Mutex::new(nonce)));
                    }
                }
            }
        }

        if accounts.is_empty() {
            return Err(eyre::eyre!("No accounts found in file"));
        }

        Ok(Self { accounts, eth_client })
    }

    /// Starts the account manager task that periodically checks account balances and processes transactions.
    pub fn start(&'static self, rt_handle: &Handle) {
        rt_handle.spawn(async move {
            loop {
                // TODO: add a listener on the pool and only try to call [`best_transaction`]
                // TODO: when we are sure there is a transaction in the pool. This avoids an
                // TODO: constant loop which rarely yields to the executor combined with a
                // TODO: sleep which could sleep for a while before handling transactions.
                let best_hashes =
                    self.eth_client.mempool().as_ref().best_transactions().map(|x| *x.hash()).collect::<Vec<_>>();
                if let Some(best_hash) = best_hashes.first() {
                    let maybe_lock = self.lock_account().await;
                    if maybe_lock.is_err() {
                        tracing::error!(target: "account_manager", err = ?maybe_lock.unwrap(), "failed to fetch relayer");
                        continue;
                    }

                    let (_account_address, mut locked_account_nonce) = maybe_lock.expect("maybe_lock is not error");

                    // TODO: here we send the transaction on the starknet network
                    // Increment account_nonce after sending a transaction
                    *locked_account_nonce = *locked_account_nonce + 1;

                    // Only release the lock once the transaction has been broadcast
                    drop(locked_account_nonce);

                    self.eth_client.mempool().as_ref().remove_transactions(vec![*best_hash]);
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    /// Returns the next available account from the manager.
    async fn lock_account(&self) -> eyre::Result<(Felt, MutexGuard<'_, Felt>)>
    where
        SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
    {
        let mut accounts = self.accounts.iter().collect::<HashMap<_, _>>();
        loop {
            if accounts.is_empty() {
                return Err(eyre::eyre!("failed to fetch funded account"));
            }
            // use [`select_all`] to poll an iterator over impl Future<Output = (Felt, MutexGuard<Felt>)>
            // We use Box::pin because this Future doesn't implement `Unpin`.
            let fut_locks = accounts.iter().map(|(address, nonce)| Box::pin(async { (*address, nonce.lock().await) }));
            let ((account_address, guard), _, _) = select_all(fut_locks).await;

            // Fetch the balance of the selected account
            let balance = self
                .get_balance(*account_address)
                .await
                .inspect_err(|err| {
                    tracing::error!(target: "account_manager", ?account_address, ?err, "failed to fetch balance");
                })
                .unwrap_or_default();

            // If the balance is lower than the threshold, continue
            if balance < U256::from(u128::pow(10, 18)) {
                accounts.remove(account_address);
                continue;
            }

            // Return the account address and the guard on the nonce
            return Ok((*account_address, guard));
        }
    }

    /// Retrieves the balance of the specified account address.
    async fn get_balance(&self, account_address: Felt) -> eyre::Result<U256> {
        // Convert the optional Ethereum block ID to a Starknet block ID.
        let starknet_block_id = self.eth_client.eth_provider().to_starknet_block_id(Some(BlockId::default())).await?;
        // Get the balance of the address at the given block ID.
        self.eth_client.starknet_provider().balance_at(account_address, starknet_block_id).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixtures::katana, katana::Katana};
    use rstest::rstest;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_account_manager_setup(#[future] katana: Katana) {
        let eth_client = katana.eth_client();

        // Create a temporary file to simulate the account JSON file
        let mut temp_file = NamedTempFile::new().unwrap();
        let json_data = json!([
            {"address": "2883640181176136234335915321173609316686035330597712783682502353343947167672"},
            {"address": "163189206500119404227396165700254790683726361202744501915894563246389642629"}
        ]);
        write!(temp_file, "{json_data}").unwrap();

        // Create an AccountManager instance with the temporary file
        let account_manager =
            AccountManager::new(temp_file.path().to_str().unwrap(), Arc::new(eth_client)).await.unwrap();

        // Verify that the accounts are loaded correctly
        let accounts = account_manager.accounts;
        assert_eq!(accounts.len(), 2, "Expected 2 accounts in the manager");

        // Expected account addresses.
        //
        // These are the addresses from the temporary JSON file converted to hex.
        //
        // We want to test a different init method from hex to be sure that the account manager handle the initialization of Felts correctly.
        let expected_addresses = [
            Felt::from_hex("0x660151ef6c0c8a4eda708478c8b909a8f784fd5b25c6d0f08fa9ea9957b57b8").unwrap(),
            Felt::from_hex("0x5c5ca015b2dbfa8a25113a9e89fe996211f25a32887d43b5e9afefa3b8c585").unwrap(),
        ];

        // Validate if the accounts are initialized with the correct nonce values
        for (account, nonce) in accounts.iter() {
            // Assert that the account address is in the expected list
            assert!(expected_addresses.contains(account), "Account address should be in the expected list");
            // Assert that the account nonce is initialized to 0
            assert_eq!(*(nonce.lock().await), Felt::ZERO, "Account nonce should be initialized to 0");
        }
    }
}
