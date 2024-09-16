use crate::into_via_wrapper;
use cainome::rs::abigen_legacy;
use dotenvy::dotenv;
use reth_primitives::{Address, B256};
use starknet::{
    core::{types::Felt, utils::get_contract_address},
    macros::selector,
};
use std::{str::FromStr, sync::LazyLock};
// Contract ABIs

pub mod account_contract {
    use super::abigen_legacy;
    abigen_legacy!(AccountContract, "./.kakarot/artifacts/account_contract.json");
}

#[allow(clippy::too_many_arguments)]
pub mod core {
    use super::{abigen_legacy, Felt};
    abigen_legacy!(KakarotCore, "./.kakarot/artifacts/kakarot.json");

    #[derive(Debug)]
    pub struct CallInput {
        pub(crate) nonce: Felt,
        pub(crate) from: Felt,
        pub(crate) to: self::Option,
        pub(crate) gas_limit: Felt,
        pub(crate) gas_price: Felt,
        pub(crate) value: Uint256,
        pub(crate) calldata: Vec<Felt>,
    }
}

fn env_var_to_field_element(var_name: &str) -> Felt {
    dotenv().ok();
    let env_var = std::env::var(var_name).unwrap_or_else(|_| panic!("Missing environment variable {var_name}"));

    Felt::from_str(&env_var).unwrap_or_else(|_| panic!("Invalid hex string for {var_name}"))
}

/// Kakarot address
pub static KAKAROT_ADDRESS: LazyLock<Felt> = LazyLock::new(|| env_var_to_field_element("KAKAROT_ADDRESS"));

/// Uninitialized account class hash
pub static UNINITIALIZED_ACCOUNT_CLASS_HASH: LazyLock<Felt> =
    LazyLock::new(|| env_var_to_field_element("UNINITIALIZED_ACCOUNT_CLASS_HASH"));

/// Ethereum send transaction selector
pub static ETH_SEND_TRANSACTION: LazyLock<Felt> = LazyLock::new(|| selector!("eth_send_transaction"));

/// Execute from outside selector
pub static EXECUTE_FROM_OUTSIDE: LazyLock<Felt> = LazyLock::new(|| selector!("execute_from_outside"));

/// Maximum number of felts in calldata
pub static MAX_FELTS_IN_CALLDATA: LazyLock<usize> = LazyLock::new(|| {
    usize::from_str(
        &std::env::var("MAX_FELTS_IN_CALLDATA")
            .unwrap_or_else(|_| panic!("Missing environment variable MAX_FELTS_IN_CALLDATA")),
    )
    .expect("Failed to parse MAX_FELTS_IN_CALLDATA")
});

pub fn get_white_listed_eip_155_transaction_hashes() -> Vec<B256> {
    std::env::var("WHITE_LISTED_EIP_155_TRANSACTION_HASHES")
        .unwrap_or_else(|_| panic!("Missing environment variable WHITE_LISTED_EIP_155_TRANSACTION_HASHES"))
        .replace(' ', "")
        .split(',')
        .map(|hash| B256::from_str(hash).unwrap())
        .collect()
}

// Kakarot utils
/// Compute the starknet address given a eth address
#[inline]
pub fn starknet_address(address: Address) -> Felt {
    let evm_address = into_via_wrapper!(address);
    get_contract_address(evm_address, *UNINITIALIZED_ACCOUNT_CLASS_HASH, &[*KAKAROT_ADDRESS, evm_address], Felt::ZERO)
}
