#![allow(clippy::significant_drop_tightening)]

use super::validate::KakarotTransactionValidator;
use crate::{
    client::EthClient,
    into_via_try_wrapper,
    providers::eth_provider::{
        constant::RPC_CONFIG, database::state::EthDatabase, starknet::relayer::LockedRelayer, BlockProvider,
    },
};
use futures::future::select_all;
use reth_chainspec::ChainSpec;
use reth_execution_types::ChangedAccount;
use reth_primitives::{basefee::calc_next_block_base_fee, Address, BlockId, IntoRecoveredTransaction, U256};
use reth_revm::DatabaseRef;
use reth_rpc_types::BlockNumberOrTag;
use reth_transaction_pool::{
    blobstore::NoopBlobStore, BlockInfo, CoinbaseTipOrdering, EthPooledTransaction, Pool, TransactionOrigin,
    TransactionPool, TransactionPoolExt,
};
use serde_json::Value;
use starknet::{
    core::types::{BlockTag, Felt},
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use std::{collections::HashMap, fs::File, io::Read, str::FromStr, sync::Arc, time::Duration};
use tokio::{runtime::Handle, sync::Mutex};

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
                    let transaction = self.eth_client.mempool().get(best_hash);
                    if transaction.is_none() {
                        // Probably a race condition here
                        continue;
                    }
                    let transaction = transaction.expect("not None");

                    // We remove the transaction to avoid another relayer from picking it up.
                    self.eth_client.mempool().as_ref().remove_transactions(vec![*best_hash]);

                    // Spawn a task for the transaction to be sent
                    tokio::spawn(async move {
                        // Lock the relayer account
                        let maybe_relayer = self.lock_account().await;
                        if maybe_relayer.is_err() {
                            // If we fail to fetch a relayer, we need to re-insert the transaction in the pool
                            tracing::error!(target: "account_manager", err = ?maybe_relayer.unwrap(), "failed to fetch relayer");
                            let _ = self
                                .eth_client
                                .mempool()
                                .add_transaction(TransactionOrigin::Local, transaction.transaction.clone())
                                .await;
                            return;
                        }
                        let mut relayer = maybe_relayer.expect("maybe_lock is not error");

                        // Send the Ethereum transaction using the relayer
                        let transaction_signed = transaction.to_recovered_transaction().into_signed();
                        let res = relayer.relay_transaction(&transaction_signed).await;
                        if res.is_err() {
                            // If the relayer failed to relay the transaction, we need to reposition it in the mempool
                            let _ = self
                                .eth_client
                                .mempool()
                                .add_transaction(TransactionOrigin::Local, transaction.transaction.clone())
                                .await;
                            return;
                        }

                        // Increment account_nonce after sending a transaction
                        let nonce = relayer.nonce_mut();
                        *nonce = *nonce + 1;
                    });
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    /// Returns the next available account from the manager.
    pub async fn lock_account(&self) -> eyre::Result<LockedRelayer<'_, JsonRpcClient<HttpTransport>>>
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

            let balance = into_via_try_wrapper!(balance)?;
            let account = LockedRelayer::new(
                guard,
                *account_address,
                balance,
                JsonRpcClient::new(HttpTransport::new(RPC_CONFIG.network_url.clone())),
                self.eth_client.starknet_provider().chain_id().await.expect("Failed to get chain id"),
            );

            // Return the account address and the guard on the nonce
            return Ok(account);
        }
    }

    /// Retrieves the balance of the specified account address for the [`BlockTag::Pending`]
    async fn get_balance(&self, account_address: Felt) -> eyre::Result<U256> {
        // Get the balance of the address for the Pending block.
        self.eth_client
            .starknet_provider()
            .balance_at(account_address, starknet::core::types::BlockId::Tag(BlockTag::Pending))
            .await
            .map_err(Into::into)
    }
}

#[derive(Default)]
struct LoadedAccounts {
    /// All accounts that were loaded
    accounts: Vec<ChangedAccount>,
    /// All accounts that failed to load
    failed_to_load: Vec<Address>,
}

/// Loads all accounts at the given state
///
/// Note: this expects _unique_ addresses
fn load_accounts<SP, I>(client: &Arc<EthClient<SP>>, addresses: I) -> LoadedAccounts
where
    SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
    I: IntoIterator<Item = Address>,
{
    let addresses = addresses.into_iter();
    let mut res = LoadedAccounts::default();

    let db = EthDatabase::new(Arc::new(client.eth_provider()), BlockNumberOrTag::Latest.into());

    for addr in addresses {
        if let Ok(maybe_acc) = db.basic_ref(addr) {
            let acc = maybe_acc.map_or_else(
                || ChangedAccount::empty(addr),
                |acc| ChangedAccount { address: addr, nonce: acc.nonce, balance: acc.balance },
            );
            res.accounts.push(acc);
        } else {
            // failed to load account.
            res.failed_to_load.push(addr);
        }
    }
    res
}

pub fn maintain_transaction_pool<SP>(eth_client: Arc<EthClient<SP>>, rt_handle: &Handle)
where
    SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
{
    rt_handle.spawn(async move {
        loop {
            // ensure the pool points to latest state
            if let Ok(Some(latest)) = eth_client.eth_provider().header(&BlockNumberOrTag::Latest.into()).await {
                let chain_spec = ChainSpec { chain: eth_client.eth_provider().chain_id.into(), ..Default::default() };
                let info = BlockInfo {
                    block_gas_limit: latest.gas_limit as u64,
                    last_seen_block_hash: latest.hash,
                    last_seen_block_number: latest.number,
                    pending_basefee: calc_next_block_base_fee(
                        latest.gas_used,
                        latest.gas_limit,
                        latest.base_fee_per_gas.unwrap_or_default(),
                        chain_spec.base_fee_params_at_timestamp(latest.timestamp + 12),
                    ) as u64,
                    pending_blob_fee: latest.next_block_blob_fee(),
                };
                eth_client.mempool().set_block_info(info);
            }

            // Fetch unique senders from the mempool that are out of sync
            let dirty_addresses = eth_client.mempool().unique_senders();

            // if we have accounts that are out of sync with the pool, we reload them in chunks
            if !dirty_addresses.is_empty() {
                // can fetch all dirty accounts at once
                let reloaded = load_accounts(&eth_client.clone(), dirty_addresses);
                // update the pool with the loaded accounts
                eth_client.mempool().update_accounts(reloaded.accounts);
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
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
        for (account, nonce) in &accounts {
            // Assert that the account address is in the expected list
            assert!(expected_addresses.contains(account), "Account address should be in the expected list");
            // Assert that the account nonce is initialized to 0
            assert_eq!(*(nonce.lock().await), Felt::ZERO, "Account nonce should be initialized to 0");
        }
    }
}
