use std::fs;

use dojo_test_utils::rpc::MockJsonRpcTransport;
use foundry_config::find_git_root_path;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::{JsonRpcClient, SequencerGatewayProvider};
use starknet_crypto::FieldElement;
use walkdir::WalkDir;

use super::constants::{KAKAROT_ADDRESS, KAKAROT_TESTNET_ADDRESS, PROXY_ACCOUNT_CLASS_HASH};
use crate::client::config::{Network, SequencerGatewayProviderBuilder, StarknetConfig};
use crate::client::KakarotClient;

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

/// Returns all the fixtures present in the fixtures directory.
/// # Panics
/// Returns an error if a fixture is not a valid json or if the fixture is not present in both
/// requests and responses.
///
/// Panics if `all_fixtures` is called from a directory where `core` crate is not in scope.
pub fn all_fixtures() -> Vec<StarknetRpcFixture> {
    let cwd = &std::env::current_dir().unwrap();
    let request_path = find_git_root_path(cwd).unwrap().join("crates/core/src/mock/fixtures/requests");
    let mut fixtures: Vec<StarknetRpcFixture> = Vec::new();
    for entry in WalkDir::new(request_path) {
        let entry = entry.unwrap();
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let request = fs::read_to_string(path).unwrap().parse::<Value>().unwrap();
        // We need to clean the escape characters, we get the following error:
        // panicked at 'Response not set in mock for method
        // "\"starknet_getTransactionByBlockIdAndIndex\"" and params "[{\"block_hash\": [...]
        let params = request["params"].clone();
        let method_name = request["method"].clone();
        let response = fs::read_to_string(path.to_string_lossy().replace("requests", "responses"))
            .unwrap()
            .parse::<Value>()
            .unwrap();

        fixtures.push(StarknetRpcFixture { method: serde_json::from_value(method_name).unwrap(), params, response });
    }
    fixtures
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

pub fn init_testnet_client() -> KakarotClient<SequencerGatewayProvider> {
    let kakarot_address = FieldElement::from_hex_be(KAKAROT_TESTNET_ADDRESS).unwrap();
    let config = StarknetConfig::new(Network::Goerli1Gateway, kakarot_address, Default::default());

    let provider = SequencerGatewayProviderBuilder::new(&Network::Goerli1Gateway).build();
    KakarotClient::new(config, provider)
}

pub fn init_mock_client(
    fixtures: Option<Vec<StarknetRpcFixture>>,
) -> KakarotClient<JsonRpcClient<MockJsonRpcTransport>> {
    let config = StarknetConfig::new(Network::Katana, *KAKAROT_ADDRESS, *PROXY_ACCOUNT_CLASS_HASH);
    let starknet_provider = mock_starknet_provider(fixtures);

    KakarotClient::new(config, starknet_provider)
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

    #[test]
    fn test_rpc_all_fixtures() {
        let fixtures = all_fixtures();
        assert_eq!(fixtures.len(), 15);
    }
}
