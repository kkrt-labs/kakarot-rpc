use std::fs;

use dojo_test_utils::rpc::MockJsonRpcTransport;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::JsonRpcClient;

/// A fixture for a Starknet RPC call.
pub struct StarknetRpcFixture {
    /// The method to call.
    method: JsonRpcMethod,
    /// The params to call the method with.
    params: Value,
    /// The response to return.
    response: Value,
}

#[derive(Debug, Deserialize)]
pub enum AvailableFixtures {
    ComputeStarknetAddress,
    GetEvmAddress,
    GetClassHashAt(String, String),
    Other(JsonRpcMethod),
}

impl From<AvailableFixtures> for JsonRpcMethod {
    fn from(value: AvailableFixtures) -> Self {
        match value {
            AvailableFixtures::Other(method) => method,
            AvailableFixtures::ComputeStarknetAddress | AvailableFixtures::GetEvmAddress => JsonRpcMethod::Call,
            AvailableFixtures::GetClassHashAt(_, _) => JsonRpcMethod::GetClassHashAt,
        }
    }
}

impl Serialize for AvailableFixtures {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AvailableFixtures::ComputeStarknetAddress => serializer.serialize_str("kakarot_computeStarknetAddress"),
            AvailableFixtures::GetEvmAddress => serializer.serialize_str("kakarot_getEvmAddress"),
            AvailableFixtures::GetClassHashAt(_, _) => serializer.serialize_str("starknet_getClassHashAt"),
            AvailableFixtures::Other(method) => method.serialize(serializer),
        }
    }
}

#[macro_export]
macro_rules! wrap_kakarot {
    ($id:expr) => {
        AvailableFixtures::Other($id)
    };
}

/// A builder for a `StarknetRpcFixture`.
pub struct StarknetRpcFixtureBuilder {
    /// The Kakarot Json RPC method.
    method: AvailableFixtures,
    /// The fixture to build.
    fixture: StarknetRpcFixture,
    /// The request loaded.
    request: Value,
    /// The response loaded.
    response: Value,
}

impl StarknetRpcFixtureBuilder {
    /// Returns a new `StarknetRpcFixtureBuilder`.
    pub fn new(method: AvailableFixtures) -> Self {
        Self {
            method,
            fixture: StarknetRpcFixture {
                method: JsonRpcMethod::BlockNumber,
                params: Value::Null,
                response: Value::Null,
            },
            request: Value::Null,
            response: Value::Null,
        }
    }

    /// Loads the request and response from the fixtures directory.
    pub fn load_jsons(mut self) -> Self {
        let clean_quotations = |s: &str| s.replace('\"', "");
        let request_path = format!(
            "src/mock/fixtures/requests/{}.json",
            clean_quotations(&serde_json::to_string(&self.method).unwrap())
        );
        let response_path = format!(
            "src/mock/fixtures/responses/{}.json",
            clean_quotations(&serde_json::to_string(&self.method).unwrap())
        );

        self.request = fs::read_to_string(request_path).unwrap().parse::<Value>().unwrap();
        self.response = fs::read_to_string(response_path).unwrap().parse::<Value>().unwrap();

        self
    }

    /// Sets the params of the fixture.
    pub fn with_params(mut self) -> Self {
        self.fixture.params = self.request["params"].clone();
        // Add the address to the params if call to get_class_hash_at.
        if let AvailableFixtures::GetClassHashAt(address, _) = &self.method {
            self.fixture.params = serde_json::json!(["latest", address]);
        }
        self
    }

    /// Sets the response of the fixture.
    pub fn with_response(mut self) -> Self {
        self.fixture.response = self.response.clone();
        // Add the class hash to the response if call to get_class_hash_at.
        if let AvailableFixtures::GetClassHashAt(_, class_hash) = &self.method {
            self.fixture.response["result"] = serde_json::json!(class_hash);
        }
        self
    }

    /// Build the `StarknetRpcFixture`.
    pub fn build(self) -> StarknetRpcFixture {
        let mut fixture = self.fixture;
        fixture.method = self.method.into();
        fixture
    }
}

/// Iterates over the given methods and returns a vector of fixtures, loading the requests and
/// responses using the fixture builder.
///
/// # Arguments
///
/// * `methods` - The json rpc methods to create fixtures for.
pub fn fixtures(methods: Vec<AvailableFixtures>) -> Vec<StarknetRpcFixture> {
    methods
        .into_iter()
        .map(|method| StarknetRpcFixtureBuilder::new(method).load_jsons().with_params().with_response().build())
        .collect()
}

/// Creates a mock `JsonRpcClient` with the given fixtures.
///
/// # Arguments
///
/// * `fixtures` - The fixtures to use.
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
    use super::*;

    #[test]
    fn test_rpc_fixture_builder() {
        // Given
        let method = wrap_kakarot!(JsonRpcMethod::GetNonce);
        let fixture = StarknetRpcFixtureBuilder::new(method).load_jsons().with_params().with_response().build();

        // When
        let expected_params = serde_json::json!(["latest", "0xabde1"]);
        let expected_response = serde_json::json!({
          "id": 1,
          "result": "0x1"
        });

        // Then
        assert_eq!(expected_params, fixture.params);
        assert_eq!(expected_response, fixture.response);
    }
}
