pub const CHAIN_ID: u64 = 1_263_227_476;

pub const STARKNET_NATIVE_TOKEN: &str =
    "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";

pub mod selectors {
    use starknet::{core::types::FieldElement, macros::selector};

    pub const GET_STARKNET_CONTRACT_ADDRESS: FieldElement =
        selector!("get_starknet_contract_address");
    pub const BYTECODE: FieldElement = selector!("bytecode");

    pub const EXECUTE_AT_ADDRESS: FieldElement = selector!("execute_at_address");
    pub const COMPUTE_STARKNET_ADDRESS: FieldElement = selector!("compute_starknet_address");

    pub const GET_EVM_ADDRESS: FieldElement = selector!("get_evm_address");

    pub const BALANCE_OF: FieldElement = selector!("balanceOf");
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
