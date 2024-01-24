use std::sync::Mutex;

use kakarot_rpc::{starknet_client::constants::CHAIN_ID, storage::database::MockEthereumProvider};
use lazy_static::lazy_static;
use reth_primitives::U64;

lazy_static! {
    /// A Mutex-wrapped U64 that is used to mock the block number.
    pub static ref MOCK_BLOCK_NUMBER: Mutex<U64> = Mutex::new(U64::from(0));
}

/// Returns a MockEthereumProvider that can be used to mock the EthereumProvider.
pub fn mock_ethereum_provider() -> MockEthereumProvider {
    let mut eth_db = MockEthereumProvider::new();

    eth_db.expect_chain_id().returning(|| Box::pin(async { Ok(Some(U64::from(CHAIN_ID))) }));
    // In order to increment the block number, we use a Mutex to lock the block number and increment
    // it by 1 each time the function is called.
    eth_db.expect_block_number().returning(|| {
        Box::pin(async {
            let mut lock = MOCK_BLOCK_NUMBER.lock().unwrap();
            let block_number = *lock;
            *lock += U64::from(1);
            drop(lock);
            Ok(block_number)
        })
    });

    eth_db
}
