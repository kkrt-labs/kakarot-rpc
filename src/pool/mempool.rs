#![allow(clippy::significant_drop_tightening)]

use super::validate::KakarotTransactionValidator;
use crate::{
    client::EthClient,
    constants::KAKAROT_RPC_CONFIG,
    into_via_try_wrapper,
    pool::constants::ONE_TENTH_ETH,
    providers::eth_provider::{database::state::EthDatabase, starknet::relayer::LockedRelayer, BlockProvider},
};
use alloy_primitives::{Address, U256};
use futures::future::select_all;
use reth_chainspec::ChainSpec;
use reth_execution_types::ChangedAccount;
use reth_primitives::BlockNumberOrTag;
use reth_revm::DatabaseRef;
use reth_transaction_pool::{
    blobstore::NoopBlobStore, BlockInfo, CanonicalStateUpdate, CoinbaseTipOrdering, EthPooledTransaction, Pool,
    TransactionOrigin, TransactionPool, TransactionPoolExt,
};
use starknet::{
    core::types::{BlockTag, Felt},
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

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
    /// Initialize the account manager with a set of passed accounts.
    pub async fn from_addresses(addresses: Vec<Felt>, eth_client: Arc<EthClient<SP>>) -> eyre::Result<Self> {
        let mut accounts = HashMap::new();

        for add in addresses {
            // Query the initial account_nonce for the account from the provider
            let nonce = eth_client
                .starknet_provider()
                .get_nonce(starknet::core::types::BlockId::Tag(BlockTag::Pending), add)
                .await
                .unwrap_or_default();
            accounts.insert(add, Arc::new(Mutex::new(nonce)));
        }

        Ok(Self { accounts, eth_client })
    }

    /// Starts the account manager task that periodically checks account balances and processes transactions.
    pub fn start(self) {
        let this = Arc::new(self);

        // Start the nonce updater in a separate task
        this.clone().start_nonce_updater();

        tokio::spawn(async move {
            loop {
                // TODO: add a listener on the pool and only try to call [`best_transaction`]
                // TODO: when we are sure there is a transaction in the pool. This avoids an
                // TODO: constant loop which rarely yields to the executor combined with a
                // TODO: sleep which could sleep for a while before handling transactions.
                let best_hashes =
                    this.eth_client.mempool().as_ref().best_transactions().map(|x| *x.hash()).collect::<Vec<_>>();
                if let Some(best_hash) = best_hashes.first() {
                    let transaction = this.eth_client.mempool().get(best_hash);
                    if transaction.is_none() {
                        // Probably a race condition here
                        continue;
                    }
                    let transaction = transaction.expect("not None");

                    // We remove the transaction to avoid another relayer from picking it up.
                    this.eth_client.mempool().as_ref().remove_transactions(vec![*best_hash]);

                    // Spawn a task for the transaction to be sent
                    let manager = this.clone();
                    tokio::spawn(async move {
                        // Lock the relayer account
                        let maybe_relayer = manager.lock_account().await;
                        if maybe_relayer.is_err() {
                            // If we fail to fetch a relayer, we need to re-insert the transaction in the pool
                            tracing::error!(target: "account_manager", err = ?maybe_relayer.unwrap_err(), "failed to fetch relayer");
                            let _ = manager
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
                            tracing::error!(target: "account_manager", err = ?res.unwrap_err(), "failed to relay transaction");
                            let _ = manager
                                .eth_client
                                .mempool()
                                .add_transaction(TransactionOrigin::Local, transaction.transaction.clone())
                                .await;
                            return;
                        }

                        tracing::info!(target: "account_manager", starknet_hash = ?res.expect("not error"), ethereum_hash = ?transaction_signed.hash());

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
            let balance = self.get_balance(*account_address).await?;

            // If the balance is lower than the threshold, continue
            if balance < U256::from(ONE_TENTH_ETH) {
                accounts.remove(account_address);
                continue;
            }

            let balance = into_via_try_wrapper!(balance)?;
            let chain_id = self.eth_client.starknet_provider().chain_id().await?;

            let account = LockedRelayer::new(
                guard,
                *account_address,
                balance,
                JsonRpcClient::new(HttpTransport::new(KAKAROT_RPC_CONFIG.network_url.clone())),
                chain_id,
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

    /// Update the nonces for all accounts every minute.
    pub fn start_nonce_updater(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                for (address, nonce_mutex) in &self.accounts {
                    // Query the updated nonce for the account from the provider
                    let new_nonce = self
                        .eth_client
                        .starknet_provider()
                        .get_nonce(starknet::core::types::BlockId::Tag(BlockTag::Pending), *address)
                        .await
                        .unwrap_or_default();

                    let mut nonce = nonce_mutex.lock().await;
                    *nonce = new_nonce;

                    tracing::info!(target: "account_manager", account = ?address, new_nonce = ?new_nonce, "updated account nonce");
                }

                // Sleep for 1 minute before the next update
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
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

/// Maintains the transaction pool by periodically polling the database in order to
/// fetch the latest block and mark the block's transactions as mined by the node.
pub fn maintain_transaction_pool<SP>(eth_client: Arc<EthClient<SP>>)
where
    SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
{
    tokio::spawn(async move {
        let mut block_number = 0u64;
        loop {
            // Fetch the latest block number
            let Ok(current_block_number) = eth_client.eth_provider().block_number().await else {
                tracing::error!(target: "maintain_transaction_pool", "failed to fetch current block number");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };

            if current_block_number.to::<u64>() > block_number {
                // Fetch the block by number for the latest block
                if let Ok(Some(latest_block)) =
                    eth_client.eth_provider().block_by_number(BlockNumberOrTag::Latest, true).await
                {
                    let hash = latest_block.header.hash;

                    // If we can convert the RPC block to a primitive block, we proceed
                    if let Ok(latest_block) = TryInto::<reth_primitives::Block>::try_into(latest_block.inner) {
                        let latest_header = latest_block.header.clone().seal(hash);

                        // Update the block information in the pool
                        let chain_spec =
                            ChainSpec { chain: eth_client.eth_provider().chain_id.into(), ..Default::default() };
                        let info = BlockInfo {
                            block_gas_limit: latest_header.gas_limit,
                            last_seen_block_hash: hash,
                            last_seen_block_number: latest_header.number,
                            pending_basefee: latest_header
                                .next_block_base_fee(
                                    chain_spec.base_fee_params_at_timestamp(latest_header.timestamp + 12),
                                )
                                .unwrap_or_default(),
                            pending_blob_fee: latest_header.next_block_blob_fee(),
                        };
                        eth_client.mempool().set_block_info(info);

                        // Fetch unique senders from the mempool that are out of sync
                        let dirty_addresses = eth_client.mempool().unique_senders();

                        let mut changed_accounts = Vec::new();

                        // if we have accounts that are out of sync with the pool, we reload them in chunks
                        if !dirty_addresses.is_empty() {
                            // can fetch all dirty accounts at once
                            let reloaded = load_accounts(&eth_client.clone(), dirty_addresses);
                            changed_accounts.extend(reloaded.accounts);
                            // update the pool with the loaded accounts
                            eth_client.mempool().update_accounts(changed_accounts.clone());
                        }

                        let sealed_block = latest_block.seal(hash);
                        let mined_transactions = sealed_block.body.transactions.iter().map(|tx| tx.hash).collect();

                        // Canonical update
                        let update = CanonicalStateUpdate {
                            new_tip: &sealed_block,
                            pending_block_base_fee: info.pending_basefee,
                            pending_block_blob_fee: None,
                            changed_accounts,
                            mined_transactions,
                        };
                        eth_client.mempool().on_canonical_state_change(update);

                        block_number = current_block_number.to();
                    } else {
                        tracing::error!(target: "maintain_transaction_pool", "failed to convert block");
                    }
                } else {
                    tracing::error!(target: "maintain_transaction_pool", "failed to fetch latest block");
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
