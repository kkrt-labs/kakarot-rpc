use dojo_test_utils::rpc::MockJsonRpcTransport;
use serde_json::Value;
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::JsonRpcClient;

pub struct StarknetRpcFixture {
    method: JsonRpcMethod,
    params: Value,
    response: Value,
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
