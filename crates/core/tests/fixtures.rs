use ethers::abi::Token;
use ethers::types::Address as EthersAddress;
use futures::executor::block_on;
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::models::felt::Felt252Wrapper;
use kakarot_rpc_core::test_utils::deploy_helpers::{ContractDeploymentArgs, DeployedKakarot, KakarotTestEnvironment};
use reth_primitives::Address;
use rstest::*;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;

/// This is a the context for a Kakarot test environment
/// It is a wrapper around the KakarotTestEnvironment
pub struct KakarotTestEnvironmentContext {
    test_environment: KakarotTestEnvironment,
}

pub enum TestContext {
    Simple,
    Counter,
    PlainOpcodes,
    ERC20,
}

impl KakarotTestEnvironmentContext {
    pub async fn setup(test_context: TestContext) -> Self {
        println!("async setup");

        // Create a new test environment
        let mut test_environment = KakarotTestEnvironment::new().await;

        // Deploy the evm contracts depending on the test context
        match test_context {
            TestContext::Simple => Self { test_environment },

            TestContext::Counter => {
                // Deploy the Counter contract
                test_environment = test_environment
                    .deploy_evm_contract(ContractDeploymentArgs { name: "Counter".into(), constructor_args: () })
                    .await;
                Self { test_environment }
            }
            TestContext::PlainOpcodes => {
                // Deploy the Counter contract
                test_environment = test_environment
                    .deploy_evm_contract(ContractDeploymentArgs { name: "Counter".into(), constructor_args: () })
                    .await;
                let counter = test_environment.evm_contract("Counter");
                let counter_eth_address: Address = {
                    let address: Felt252Wrapper = counter.addresses.eth_address.into();
                    address.try_into().unwrap()
                };

                // Deploy the PlainOpcodes contract
                test_environment = test_environment
                    .deploy_evm_contract(ContractDeploymentArgs {
                        name: "PlainOpcodes".into(),
                        constructor_args: (EthersAddress::from(counter_eth_address.as_fixed_bytes()),),
                    })
                    .await;
                Self { test_environment }
            }
            TestContext::ERC20 => {
                // Deploy the ERC20 contract
                test_environment = test_environment
                    .deploy_evm_contract(ContractDeploymentArgs {
                        name: "ERC20".into(),
                        constructor_args: (
                            Token::String("Test".into()),               // name
                            Token::String("TT".into()),                 // symbol
                            Token::Uint(ethers::types::U256::from(18)), // decimals
                        ),
                    })
                    .await;
                Self { test_environment }
            }
        }
    }

    pub fn resources(
        &self,
    ) -> (&KakarotTestEnvironment, &KakarotClient<JsonRpcClient<HttpTransport>>, &DeployedKakarot) {
        (&self.test_environment, self.test_environment.client(), self.test_environment.kakarot())
    }
}

impl Drop for KakarotTestEnvironmentContext {
    fn drop(&mut self) {
        block_on(async move { println!("async teardown") })
    }
}

#[fixture]
pub fn kakarot_test_env_ctx(
    #[default(TestContext::Simple)] test_context: TestContext,
) -> KakarotTestEnvironmentContext {
    block_on(async { KakarotTestEnvironmentContext::setup(test_context).await })
}
