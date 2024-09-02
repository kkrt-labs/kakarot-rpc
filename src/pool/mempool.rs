use super::validate::KakarotTransactionValidator;
use crate::{
    into_via_wrapper,
    models::felt::Felt252Wrapper,
    pool::{filter, EthDatabaseFilterBuilder},
    providers::eth_provider::{
        database::types::oz_account::StoredOzAccount,
        error::ExecutionError,
        provider::EthDataProvider,
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
use std::{collections::HashSet, fs::File, io::Read, time::Duration};
use tokio::{runtime::Handle, task::JoinHandle};
use tracing::Instrument;

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

    pub fn start<SP>(&self, rt_handle: &Handle, eth_provider: &'static EthDataProvider<SP>) -> eyre::Result<()>
    where
        SP: starknet::providers::Provider + Send + Sync + 'static,
    {
        let accounts = self.accounts.clone();

        let handle: JoinHandle<Result<(), eyre::Report>> = rt_handle.spawn(async move {
            loop {
                for address in &accounts {
                    let account_filter =
                        EthDatabaseFilterBuilder::<filter::OzAccount>::default().with_oz_address(address).build();

                    let oz_account = eth_provider
                        .database()
                        .get_one::<StoredOzAccount>(account_filter.clone(), None)
                        .await?
                        .ok_or_else(|| eyre::eyre!("Account not found in the database"))?;

                    // Convert the optional Ethereum block ID to a Starknet block ID.
                    let starknet_block_id = eth_provider.to_starknet_block_id(Some(BlockId::default())).await?;

                    // Create a new `ERC20Reader` instance for the Starknet native token
                    let eth_contract = ERC20Reader::new(*STARKNET_NATIVE_TOKEN, eth_provider.starknet_provider());

                    // Call the `balanceOf` method on the contract for the given address and block ID, awaiting the result
                    let span = tracing::span!(tracing::Level::INFO, "sn::balance");
                    let res = eth_contract.balanceOf(address).block_id(starknet_block_id).call().instrument(span).await;

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

                    if oz_account.current_tx_hash.is_none() && balance > U256::from(u128::pow(10, 18)) {
                        Self::process_transaction(address, eth_provider);
                    }
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        rt_handle.spawn(async {
            if let Err(e) = handle.await {
                tracing::error!("Error in spawned task: {:?}", e);
            }
        });

        Ok(())
    }

    fn process_transaction<SP>(_address: &Felt, eth_provider: &EthDataProvider<SP>)
    where
        SP: starknet::providers::Provider + Send + Sync + 'static,
    {
        let best_hashes =
            eth_provider.mempool.as_ref().unwrap().best_transactions().map(|x| *x.hash()).collect::<Vec<_>>();

        if let Some(best_hash) = best_hashes.first() {
            eth_provider.mempool.as_ref().unwrap().remove_transactions(vec![*best_hash]);
        }
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
