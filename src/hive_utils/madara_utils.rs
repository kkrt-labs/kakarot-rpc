use crate::starknet_client::constants::STARKNET_NATIVE_TOKEN;
use crate::starknet_client::helpers::split_u256_into_field_elements;
use reth_primitives::{Bytes, U256};
use starknet::core::types::FieldElement;
use starknet::core::utils::get_storage_var_address;

use crate::hive_utils::types::{ContractAddress, StorageKey, StorageValue};

/// Generates the genesis storage tuples for setting the bytecode of a Kakarot countract account
///
/// This function calculates the storage keys for the Kakarot contract using the provided bytecode
/// and Starknet address. The resulting Vec of tuples represent the initial storage of the Kakarot
/// contract, where the storage key is computed using the storage variable "bytecode_" and the index
/// of the 16-byte chunk of the bytecode. The value stored is the 16-byte chunk of the bytecode.
pub fn genesis_set_bytecode(
    bytecode: &Bytes,
    starknet_address: FieldElement,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    bytecode
        .chunks(16)
        .enumerate()
        .map(|(i, x)| {
            let mut storage_value = [0u8; 16];
            storage_value[..x.len()].copy_from_slice(x);
            let storage_value = FieldElement::from(u128::from_be_bytes(storage_value));

            genesis_set_storage_starknet_contract(
                starknet_address,
                "bytecode_",
                &[FieldElement::from(i)],
                storage_value,
                0, // only felt is stored so offset is always 0
            )
        })
        .collect()
}

/// Generates the genesis storage tuple for setting the storage of a Starknet contract.
///
/// This function calculates the storage key for the storage variable `storage_variable_name` and
/// its keys. The resulting tuple represents the initial storage of the contract, where the storage
/// key at a given `storage_offset` is set to the specified `storage_value`.
pub fn genesis_set_storage_starknet_contract(
    starknet_address: FieldElement,
    storage_variable_name: &str,
    keys: &[FieldElement],
    storage_value: FieldElement,
    storage_offset: u64,
) -> ((ContractAddress, StorageKey), StorageValue) {
    // Compute the storage key for the storage variable name and the keys.
    let mut storage_key =
        get_storage_var_address(storage_variable_name, keys).expect("Non-ASCII storage variable name");

    // Add the offset to the storage key.
    storage_key += FieldElement::from(storage_offset);

    let contract_address: ContractAddress = starknet_address.into();

    // Create the tuple for the initial storage data on the Starknet contract with the given storage
    // key.
    ((contract_address, storage_key.into()), storage_value.into())
}

/// Generates the genesis storage tuples for pre-funding a Starknet address on Madara.
///
/// This function calculates the storage keys for the balance of the ERC20 Fee Token
/// contract using the provided Starknet address. The resulting Vec of tuples represent the initial
/// storage of the Fee Token contract, where the account associated with the Starknet address is
/// pre-funded with the specified `amount`. The `amount` is split into two 128-bit chunks, which
/// are stored in the storage keys at offsets 0 and 1.
pub fn genesis_fund_starknet_address(
    starknet_address: FieldElement,
    amount: U256,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    // Split the amount into two 128-bit chunks.
    let amount = split_u256_into_field_elements(amount);

    // Iterate over the storage key offsets and generate the storage tuples.
    amount
        .iter()
        .enumerate() // Enumerate the key offsets.
        .map(|(offset, value)| {
            genesis_set_storage_starknet_contract(
                FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap(), // Safe unwrap
                "ERC20_balances",
                &[starknet_address],
                *value,
                offset as u64,
            )
        })
        .collect()
}

/// Generates the genesis storage tuples for a given amount of allowance to Kakarot of a Starknet
/// address on Madara.
pub fn genesis_approve_kakarot(
    starknet_address: FieldElement,
    kakarot_address: FieldElement,
    amount: U256,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    // Split the amount into two 128-bit chunks.
    let amount = split_u256_into_field_elements(amount);

    // Iterate over the storage key offsets and generate the storage tuples.
    amount
        .iter()
        .enumerate() // Enumerate the key offsets.
        .map(|(offset, value)| {
            genesis_set_storage_starknet_contract(
                FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap(), // Safe unwrap
                "ERC20_allowances",
                &[starknet_address, kakarot_address],
                *value,
                offset as u64,
            )
        })
        .collect()
}

/// Generates the genesis storage tuples for setting the storage of the Kakarot contract.
///
/// This function calculates the storage keys for the Kakarot contract using the provided Starknet
/// address. The resulting Vec of tuples represent the initial storage of the Kakarot contract,
/// where the storage key is computed using the provided `key` of the storage variable "storage_"
/// and the `value` is split into two 128-bit chunks, which are stored in the storage keys at
/// offsets 0 and 1.
pub fn genesis_set_storage_kakarot_contract_account(
    starknet_address: FieldElement,
    key: U256,
    value: U256,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    // Split the key into Vec of two 128-bit chunks.
    let keys = split_u256_into_field_elements(key);

    // Split the value into two 128-bit chunks.
    let values = split_u256_into_field_elements(value);

    // Iterate over the storage key offsets and generate the storage tuples.
    values
        .iter()
        .enumerate() // Enumerate the key offsets.
        .map(|(offset, value)| {
            genesis_set_storage_starknet_contract(
                starknet_address,
                "storage_",
                &keys,
                *value,
                offset as u64,
            )
        })
        .collect()
}
