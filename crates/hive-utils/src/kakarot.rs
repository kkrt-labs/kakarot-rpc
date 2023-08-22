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

#[cfg(test)]
mod tests {
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS;
    use kakarot_rpc_core::test_utils::deploy_helpers::KakarotTestEnvironmentContext;
    use kakarot_rpc_core::test_utils::fixtures::kakarot_test_env_ctx;
    use rstest::*;
    use starknet::core::types::{BlockId, BlockTag};

    use super::compute_starknet_address;

    /// This test is done against the Kakarot system deployed on the Starknet test sequencer.
    /// It tests the compute_starknet_address function by comparing the result of the computation
    /// with the result when called on the deployed Kakarot contract.
    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_compute_starknet_address(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        let deployed_kakarot = kakarot_test_env_ctx.kakarot();
        let kakarot_contract = kakarot_test_env_ctx.kakarot_contract();

        // Define the EVM address to be used for calculating the Starknet address
        let evm_address = *ACCOUNT_ADDRESS;

        // Calculate the Starknet address
        let starknet_address =
            compute_starknet_address(deployed_kakarot.kakarot_address, deployed_kakarot.proxy_class_hash, evm_address);

        // Calculate the expected Starknet address
        let expected_starknet_address =
            kakarot_contract.compute_starknet_address(&evm_address, &BlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Assert that the calculated Starknet address matches the expected Starknet address
        assert_eq!(starknet_address, expected_starknet_address, "Starknet address does not match");
    }
}
