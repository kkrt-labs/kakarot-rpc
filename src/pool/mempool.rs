use super::validate::KakarotTransactionValidator;
use crate::{
    client::EthClient,
    into_via_wrapper,
    models::felt::Felt252Wrapper,
    providers::eth_provider::{
        error::ExecutionError,
        starknet::{ERC20Reader, STARKNET_NATIVE_TOKEN},
        utils::{class_hash_not_declared, contract_not_found},
    },
};
use reth_primitives::{BlockId, U256};
use reth_transaction_pool::{
    blobstore::NoopBlobStore, CoinbaseTipOrdering, EthPooledTransaction, Pool, TransactionPool,
};
use serde_json::Value;
use starknet::core::types::Felt;
use std::{collections::HashMap, fs::File, io::Read, sync::Arc, time::Duration};
use tokio::{runtime::Handle, sync::Mutex};
use tracing::Instrument;

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
    accounts: Arc<Mutex<HashMap<Felt, Felt>>>,
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
                    accounts.insert(
                        felt_address,
                        eth_client
                            .eth_provider()
                            .starknet_provider()
                            .get_nonce(starknet_block_id, felt_address)
                            .await
                            .unwrap_or_default(),
                    );
                }
            }
        }

        if accounts.is_empty() {
            return Err(eyre::eyre!("No accounts found in file"));
        }

        Ok(Self { accounts: Arc::new(Mutex::new(accounts)), eth_client })
    }

    /// Starts the account manager task that periodically checks account balances and processes transactions.
    #[allow(clippy::significant_drop_tightening)]
    pub fn start(&'static self, rt_handle: &Handle) {
        let accounts = self.accounts.clone();

        rt_handle.spawn(async move {
            loop {
                // Get account addresses first without acquiring the lock
                let account_addresses: Vec<Felt> = {
                    let accounts = accounts.lock().await;
                    accounts.keys().copied().collect()
                };

                // Iterate over account addresses and check balances
                for account_address in account_addresses {
                    // Fetch the balance and handle errors functionally
                    let balance = self
                        .get_balance(&account_address)
                        .await
                        .inspect_err(|err| {
                            tracing::error!(
                                "Error getting balance for account_address {:?}: {:?}",
                                account_address,
                                err
                            );
                        })
                        .unwrap_or_default();

                    if balance > U256::from(u128::pow(10, 18)) {
                        // Acquire lock only when necessary to modify account state
                        let mut accounts = accounts.lock().await;
                        if let Some(account_nonce) = accounts.get_mut(&account_address) {
                            self.process_transaction(&account_address, account_nonce);
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    /// Retrieves the balance of the specified account address.
    async fn get_balance(&self, account_address: &Felt) -> eyre::Result<U256> {
        // Convert the optional Ethereum block ID to a Starknet block ID.
        let starknet_block_id = self.eth_client.eth_provider().to_starknet_block_id(Some(BlockId::default())).await?;

        // Create a new `ERC20Reader` instance for the Starknet native token
        let eth_contract = ERC20Reader::new(*STARKNET_NATIVE_TOKEN, self.eth_client.eth_provider().starknet_provider());

        // Call the `balanceOf` method on the contract for the given account_address and block ID, awaiting the result
        let span = tracing::span!(tracing::Level::INFO, "sn::balance");
        let res = eth_contract.balanceOf(account_address).block_id(starknet_block_id).call().instrument(span).await;

        if contract_not_found(&res) || class_hash_not_declared(&res) {
            return Err(eyre::eyre!("Contract not found or class hash not declared"));
        }

        // Otherwise, extract the balance from the result, converting any errors to ExecutionError
        let balance = res.map_err(ExecutionError::from)?.balance;

        // Convert the low and high parts of the balance to U256
        let low: U256 = into_via_wrapper!(balance.low);
        let high: U256 = into_via_wrapper!(balance.high);

        // Combine the low and high parts to form the final balance and return it
        let balance = low + (high << 128);

        Ok(balance)
    }

    /// Processes a transaction for the given account if the balance is sufficient.
    fn process_transaction(&self, _account_address: &Felt, account_nonce: &mut Felt)
    where
        SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
    {
        let best_hashes = self.eth_client.mempool().as_ref().best_transactions().map(|x| *x.hash()).collect::<Vec<_>>();

        if let Some(best_hash) = best_hashes.first() {
            self.eth_client.mempool().as_ref().remove_transactions(vec![*best_hash]);

            // Increment account_nonce after sending a transaction
            *account_nonce = *account_nonce + 1;
        }
    }
}
