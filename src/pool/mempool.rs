#![allow(clippy::significant_drop_tightening)]

use super::validate::KakarotTransactionValidator;
use crate::{
    client::EthClient,
    constants::KAKAROT_RPC_CONFIG,
    into_via_try_wrapper,
    pool::constants::ONE_TENTH_ETH,
    providers::eth_provider::{database::state::EthDatabase, starknet::relayer::Relayer, BlockProvider},
};
use alloy_primitives::{Address, U256};
use rand::{seq::SliceRandom, SeedableRng};
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
use tokio::time::Instant;
use tracing::instrument;

/// A type alias for the Kakarot Transaction Validator.
/// Uses the Reth implementation [`TransactionValidationTaskExecutor`].
pub type Validator<Client> = KakarotTransactionValidator<Client, EthPooledTransaction>;

/// A type alias for the Kakarot Transaction Ordering.
/// Uses the Reth implementation [`CoinbaseTipOrdering`].
pub type TransactionOrdering = CoinbaseTipOrdering<EthPooledTransaction>;

/// A type alias for the Kakarot Sequencer Mempool.
pub type KakarotPool<Client> = Pool<Validator<Client>, TransactionOrdering, NoopBlobStore>;

/// Manages a collection of accounts addresses, interfacing with an Ethereum client.
///
/// This struct provides functionality to initialize account data from a file, monitor account balances,
/// and process transactions for accounts with sufficient balance.
#[derive(Debug)]
pub struct AccountManager<SP: starknet::providers::Provider + Send + Sync + Clone + 'static> {
    /// A collection of account addresses.
    accounts: Vec<Felt>,
    /// The Ethereum client used to interact with the blockchain.
    eth_client: Arc<EthClient<SP>>,
}

impl<SP: starknet::providers::Provider + Send + Sync + Clone + 'static> AccountManager<SP> {
    /// Initialize the account manager with a set of passed accounts.
    pub const fn new(accounts: Vec<Felt>, eth_client: Arc<EthClient<SP>>) -> Self {
        Self { accounts, eth_client }
    }

    /// Starts the account manager task that periodically checks account balances and processes transactions.
    #[instrument(skip_all, name = "mempool")]
    pub fn start(self) {
        let this = Arc::new(self);

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
                        let maybe_relayer = manager.get_relayer().await;
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
                        let relayer = maybe_relayer.expect("not error");

                        // Send the Ethereum transaction using the relayer
                        let transaction_signed = transaction.to_recovered_transaction().into_signed();

                        // Query the updated nonce for the account from the provider.
                        // Via on chain query we have the most up-to-date nonce.
                        let relayer_nonce = manager
                            .eth_client
                            .starknet_provider()
                            .get_nonce(starknet::core::types::BlockId::Tag(BlockTag::Pending), relayer.address())
                            .await
                            .unwrap_or_default();

                        let res = relayer.relay_transaction(&transaction_signed, relayer_nonce).await;
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
                    });
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    /// Returns the next available account from the manager.
    pub async fn get_relayer(&self) -> eyre::Result<Relayer<JsonRpcClient<HttpTransport>>>
    where
        SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
    {
        // Use `StdRng` instead of `ThreadRng` as it is `Send`
        let mut rng = rand::rngs::StdRng::from_entropy();

        // Shuffle indices of accounts randomly
        let mut account_indices: Vec<_> = (0..self.accounts.len()).collect();
        account_indices.shuffle(&mut rng);

        for index in account_indices {
            let account_address = self.accounts[index];

            // Retrieve the balance of the selected account
            let balance = self.get_balance(account_address).await?;

            // Skip accounts with insufficient balance
            if balance < U256::from(ONE_TENTH_ETH) {
                continue;
            }

            // Convert the balance to `Felt`
            let balance = into_via_try_wrapper!(balance)?;

            // Construct the `Relayer` with the account address and other relevant data
            let account = Relayer::new(
                account_address,
                balance,
                JsonRpcClient::new(HttpTransport::new(KAKAROT_RPC_CONFIG.network_url.clone())),
            );

            // Return the locked relayer instance
            return Ok(account);
        }

        Err(eyre::eyre!("failed to fetch funded account"))
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

/// Maintains the transaction pool by periodically polling the database in order to
/// fetch the latest block and mark the block's transactions as mined by the node.
pub fn maintain_transaction_pool<SP>(eth_client: Arc<EthClient<SP>>, prune_duration: Duration)
where
    SP: starknet::providers::Provider + Send + Sync + Clone + 'static,
{
    tokio::spawn(async move {
        let mut block_number = 0u64;

        // Mapping to store the transactions in the mempool with a timestamp to potentially prune them
        let mut mempool_transactions = HashMap::new();

        loop {
            // Adding the transactions to the mempool mapping with a timestamp
            for tx in eth_client
                .mempool()
                .queued_transactions()
                .into_iter()
                .chain(eth_client.mempool().pending_transactions())
            {
                mempool_transactions.entry(*tx.hash()).or_insert_with(Instant::now);
            }

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
                            pending_blob_fee: None,
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
                        let mut mined_transactions: Vec<_> =
                            sealed_block.body.transactions.iter().map(|tx| tx.hash).collect();

                        // Prune mined transactions from the mempool mapping
                        for tx_hash in &mined_transactions {
                            mempool_transactions.remove(tx_hash);
                        }

                        // Prune transactions that have been in the mempool for more than 5 minutes
                        let now = Instant::now();

                        for (tx_hash, timestamp) in mempool_transactions.clone() {
                            // - If the transaction has been in the mempool for more than 5 minutes
                            // - And the transaction is in the mempool right now
                            if now.duration_since(timestamp) > prune_duration && eth_client.mempool().contains(&tx_hash)
                            {
                                tracing::warn!(target: "maintain_transaction_pool", ?tx_hash, "pruning");

                                // Add the transaction to the mined transactions so that it can be pruned
                                mined_transactions.push(tx_hash);

                                // Remove the transaction from the mempool mapping
                                mempool_transactions.remove(&tx_hash);
                            }
                        }

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
