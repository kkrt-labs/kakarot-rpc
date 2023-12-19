use starknet::core::types::FieldElement;
use starknet::core::utils::get_contract_address;

/// Off-chain utility to calculate the Starknet address, given an EVM address.
///
/// This function calculates and returns the Starknet address by using the Kakarot address,
/// Account Proxy Class Hash and EVM address. It uses the get_contract_address function with EVM
/// address as salt to perform this calculation.
///
/// # Arguments
///
/// * `kakarot_address` - The Kakarot address
/// * `account_proxy_class_hash` - The Account Proxy Class Hash
/// * `evm_address` - The EVM address
///
/// # Returns
///
/// * `FieldElement` - The calculated Starknet address
pub fn compute_starknet_address(
    kakarot_address: FieldElement,
    account_proxy_class_hash: FieldElement,
    evm_address: FieldElement,
) -> FieldElement {
    get_contract_address(evm_address, account_proxy_class_hash, &[], kakarot_address)
}
