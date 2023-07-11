use lazy_static::lazy_static;
use reth_primitives::{H256, H64, U128, U256, U8};
use starknet::core::types::FieldElement;

/// CHAIN_ID = KKRT (0x4b4b5254) in ASCII
pub const CHAIN_ID: u64 = 1_263_227_476;

pub const STARKNET_NATIVE_TOKEN: &str = "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";

pub const EARLIEST_BLOCK_NUMBER: u64 = 0;

pub const MADARA_RPC_URL: &str = "http://127.0.0.1:9944";

pub const KATANA_RPC_URL: &str = "http://0.0.0.0:5050";

pub mod selectors {
    use starknet::core::types::FieldElement;
    use starknet::macros::selector;

    pub const BYTECODE: FieldElement = selector!("bytecode");

    pub const ETH_CALL: FieldElement = selector!("eth_call");
    pub const ETH_SEND_TRANSACTION: FieldElement = selector!("eth_send_transaction");
    pub const COMPUTE_STARKNET_ADDRESS: FieldElement = selector!("compute_starknet_address");

    pub const GET_EVM_ADDRESS: FieldElement = selector!("get_evm_address");

    pub const BALANCE_OF: FieldElement = selector!("balanceOf");

    pub const EVM_CONTRACT_DEPLOYED: FieldElement = selector!("evm_contract_deployed");
}

/// This module contains constants related to EVM gas fees.
pub mod gas {
    use reth_primitives::U128;

    /// The base fee for a transaction in gwei.
    ///
    /// Since Starknet does not currently have a market for gas fees
    /// TODO: Get Starknet "historical" Gas Price instead
    pub const BASE_FEE_PER_GAS: u64 = 1;

    /// The maximum priority fee for a transaction in gwei.
    ///
    /// This fee is the maximum amount a user is willing to pay to have their transaction
    /// included in a block quickly.
    ///
    /// Since Starknet does not currently have a market for gas fees, transactions are processed
    /// on a "first come first served" basis by the Sequencer.
    /// As a result, the priority fee is set to 0.
    pub const MAX_PRIORITY_FEE_PER_GAS: U128 = U128::ZERO;
}

/// This module contains error messages related to Kakarot.
pub mod error_messages {
    /// Error message when a transaction is not part of Kakarot.
    pub const INVALID_TRANSACTION_TYPE: &str = "L1Handler, Declare, Deploy and DeployAccount transactions unsupported";
}

// This module contains constants which are being used in place of real data that should be fetched
// in production.
lazy_static! {
    pub static ref GAS_LIMIT: U256 = U256::from(1_000_000u64);
    pub static ref GAS_USED: U256 = U256::from(500_000u64);
    pub static ref CUMULATIVE_GAS_USED: U256 = U256::from(1_000_000u64);
    pub static ref EFFECTIVE_GAS_PRICE: U128 = U128::from(1_000_000u64);
    pub static ref SIZE: Option<U256> = Some(U256::from(1_000_000u64));
    pub static ref MAX_FEE: FieldElement = FieldElement::from(100_000_000_000_000_000u64);
    pub static ref ESTIMATE_GAS: U256 = U256::from(100_000_000_000_000_000u64);
    pub static ref TRANSACTION_TYPE: U8 = U8::from(0);
    pub static ref NONCE: Option<H64> = Some(H64::zero());
    pub static ref MIX_HASH: H256 = H256::zero();
    pub static ref DIFFICULTY: U256 = U256::from(0);
    pub static ref TOTAL_DIFFICULTY: Option<U256> = None;
}
