use dojo_test_utils::rpc::MockJsonRpcTransport;
use serde_json::Value;
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::JsonRpcClient;

pub struct StarknetRpcFixture {
    method: JsonRpcMethod,
    params: Value,
    response: Value,
}

pub struct StarknetRpcFixtureBuilder {
    fixture: StarknetRpcFixture,
    request: Value,
    response: Value,
}

impl StarknetRpcFixtureBuilder {
    pub fn new(method: JsonRpcMethod) -> Self {
        Self {
            fixture: StarknetRpcFixture { method, params: Value::Null, response: Value::Null },
            request: Value::Null,
            response: Value::Null,
        }
    }

    pub fn load_jsons(mut self) -> Self {
        match self.fixture.method {
            JsonRpcMethod::GetNonce => {
                self.request = serde_json::from_str(include_str!("fixtures/requests/starknet_getNonce.json")).unwrap();
                self.response =
                    serde_json::from_str(include_str!("fixtures/responses/starknet_getNonce.json")).unwrap();
            }
            JsonRpcMethod::Call => {
                self.request = serde_json::from_str(include_str!("fixtures/requests/starknet_call.json")).unwrap();
                self.response = serde_json::from_str(include_str!("fixtures/responses/starknet_call.json")).unwrap();
            }
            _ => unimplemented!(),
        };
        self
    }

    pub fn with_params(mut self) -> Self {
        dbg!(&self.request["params"].clone());
        self.fixture.params = self.request["params"].clone();
        self
    }

    pub fn with_response(mut self) -> Self {
        dbg!(&self.response["result"].clone());
        self.fixture.response = self.response["result"].clone();
        self
    }

    pub fn build(self) -> StarknetRpcFixture {
        self.fixture
    }
}

pub fn fixtures(methods: Vec<JsonRpcMethod>) -> Vec<StarknetRpcFixture> {
    methods
        .into_iter()
        .map(|method| StarknetRpcFixtureBuilder::new(method).load_jsons().with_params().with_response().build())
        .collect()
}

pub fn mock_starknet_provider(fixtures: Option<Vec<StarknetRpcFixture>>) -> JsonRpcClient<MockJsonRpcTransport> {
    let mut transport = MockJsonRpcTransport::new();
    if let Some(fixtures) = fixtures {
        fixtures
            .into_iter()
            .for_each(|fixture| transport.set_response(fixture.method, fixture.params, fixture.response));
    }
    JsonRpcClient::new(transport)
}

#[cfg(test)]
mod tests {
    use reth_primitives::Address;
    use starknet::core::types::{BlockId, BlockTag};
    use starknet_crypto::FieldElement;

    use super::*;
    use crate::client::client_api::KakarotProvider;
    use crate::client::config::StarknetConfig;
    use crate::client::KakarotClient;

    #[tokio::test]
    async fn test_mock_provider_nonce() {
        // Given
        let fixtures = fixtures(vec![JsonRpcMethod::GetNonce, JsonRpcMethod::Call]);
        let provider = mock_starknet_provider(Some(fixtures));

        let config = StarknetConfig {
            kakarot_address: FieldElement::from_hex_be(
                "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            )
            .unwrap(),
            ..Default::default()
        };
        let client = KakarotClient::new_with_provider(config, provider).unwrap();

        // When
        let eth_address = Address::from_low_u64_be(0xabde1);
        let _nonce = client.nonce(eth_address, BlockId::Tag(BlockTag::Latest)).await.unwrap();
    }
}
