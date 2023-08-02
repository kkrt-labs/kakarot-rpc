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
    use std::sync::Arc;

    use kakarot_rpc_core::contracts::kakarot::KakarotContract;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS;
    use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
    use kakarot_rpc_core::test_utils::deploy_helpers::{construct_kakarot_test_sequencer, deploy_kakarot_system};
    use starknet::core::types::{BlockId, BlockTag, FieldElement};
    use starknet::providers::jsonrpc::HttpTransport as StarknetHttpTransport;
    use starknet::providers::JsonRpcClient;

    use super::compute_starknet_address;

    /// This test is done against the Kakarot system deployed on the Starknet test sequencer.
    /// It tests the compute_starknet_address function by comparing the result of the computation
    /// with the result when called on the deployed Kakarot contract.
    #[tokio::test]
    async fn test_compute_starknet_address() {
        // Construct a Starknet test sequencer
        let starknet_test_sequencer = construct_kakarot_test_sequencer().await;

        // Define the expected funded amount for the Kakarot system
        let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000").unwrap();

        // Deploy the Kakarot system
        let deployed_kakarot =
            deploy_kakarot_system(&starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

        // Create a new HTTP transport using the sequencer's URL
        let starknet_http_transport = StarknetHttpTransport::new(starknet_test_sequencer.url());

        // Create a new JSON RPC client using the HTTP transport
        let starknet_client = Arc::new(JsonRpcClient::new(starknet_http_transport));

        // Create a new Kakarot contract
        let kakarot_contract =
            KakarotContract::new(starknet_client, deployed_kakarot.kakarot, deployed_kakarot.kakarot_proxy);

        // Define the EVM address to be used for calculating the Starknet address
        let evm_address = *ACCOUNT_ADDRESS;

        // Calculate the Starknet address
        let starknet_address =
            compute_starknet_address(deployed_kakarot.kakarot, deployed_kakarot.kakarot_proxy, evm_address);

        // Calculate the expected Starknet address
        let expected_starknet_address =
            kakarot_contract.compute_starknet_address(&evm_address, &BlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Assert that the calculated Starknet address matches the expected Starknet address
        assert_eq!(starknet_address, expected_starknet_address, "Starknet address does not match");
    }
}
