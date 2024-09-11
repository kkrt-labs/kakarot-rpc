use super::validate::KakarotTransactionValidator;
use crate::client::EthClient;
use futures::future::select_all;
use reth_primitives::{BlockId, U256};
use reth_transaction_pool::{
    blobstore::NoopBlobStore, CoinbaseTipOrdering, EthPooledTransaction, Pool, TransactionPool,
};
use serde_json::Value;
use starknet::core::types::Felt;
use std::{collections::HashMap, fs::File, io::Read, sync::Arc, time::Duration};
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

        // Extract the account addresses from the JSON array
        if let Some(array) = json.as_array() {
            for item in array {
                if let Some(account_address) = item.as_str() {
                    let felt_address = Felt::from_hex_unchecked(account_address);

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
                    let (_address, mut locked_account_nonce) = self.lock_account().await;

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
    async fn lock_account(&self) -> (Felt, MutexGuard<'_, Felt>)
    where
        SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
    {
        loop {
            // use [`select_all`] to poll an iterator over impl Future<Output = (Felt, MutexGuard<Felt>)>
            // We use Box::pin because this Future doesn't implement `Unpin`.
            let fut_locks =
                self.accounts.iter().map(|(address, nonce)| Box::pin(async { (*address, nonce.lock().await) }));
            let ((account_address, guard), _, _) = select_all(fut_locks).await;

            // Fetch the balance of the selected account
            let balance = self
                .get_balance(account_address)
                .await
                .inspect_err(|err| {
                    tracing::error!(target: "account_manager", ?account_address, ?err, "failed to fetch balance");
                })
                .unwrap_or_default();

            // If the balance is lower than the threshold, continue
            if balance < U256::from(u128::pow(10, 18)) {
                continue;
            }

            // Return the account address and the guard on the nonce
            return (account_address, guard);
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
