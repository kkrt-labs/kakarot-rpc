use starknet::core::types::FieldElement;
use starknet::core::utils::get_contract_address;

/// Computes the StarkNet address of EVM address given the Kakarot address and Account Proxy Class
/// Hash. Uses the same logic as get_contract_address with EVM as salt.
pub fn compute_starknet_address(
    kakarot_address: FieldElement,
    account_proxy_class_hash: FieldElement,
    evm_address: FieldElement,
) -> FieldElement {
    let constructor_calldata: Vec<FieldElement> = Vec::new();
    get_contract_address(evm_address, account_proxy_class_hash, &constructor_calldata, kakarot_address)
}

#[cfg(test)]
mod tests {
    use starknet::core::crypto::compute_hash_on_elements;
    use starknet::core::types::FieldElement;
    use starknet::core::utils::normalize_address;

    use super::compute_starknet_address;

    #[test]
    fn test_compute_starknet_address() {
        // From https://github.com/xJonathanLEI/starknet-rs/blob/283c2bb814a964a49d67aafadc4340c5fb714fdd/starknet-core/src/utils.rs#L12
        // Cairo string of "STARKNET_CONTRACT_ADDRESS"
        const CONTRACT_ADDRESS_PREFIX: FieldElement = FieldElement::from_mont([
            3829237882463328880,
            17289941567720117366,
            8635008616843941496,
            533439743893157637,
        ]);

        // Given
        let kakarot_address = FieldElement::from_hex_be("0x1").unwrap();
        let account_proxy_class_hash = FieldElement::from_hex_be("0x2").unwrap();
        let evm_address = FieldElement::from_hex_be("0x3").unwrap();
        let constructor_calldata: Vec<FieldElement> = Vec::new();

        // Compute StarkNet address
        let starknet_address = compute_starknet_address(kakarot_address, account_proxy_class_hash, evm_address);

        // Construct elements to be hashed using evm_address as salt
        let elements = vec![
            CONTRACT_ADDRESS_PREFIX,
            kakarot_address,
            evm_address,
            account_proxy_class_hash,
            compute_hash_on_elements(&constructor_calldata),
        ];

        // Compute expected contract address
        let expected_starknet_address = normalize_address(compute_hash_on_elements(&elements));

        // Assert that computed and expected results are the same
        assert_eq!(starknet_address, expected_starknet_address, "Failed to get StarkNet address");
    }
}
