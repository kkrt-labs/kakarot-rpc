use dojo_test_utils::rpc::MockJsonRpcTransport;
use starknet::providers::JsonRpcClient;

pub fn mock_starknet_provider() -> JsonRpcClient<MockJsonRpcTransport> {
    let transport = MockJsonRpcTransport::new();
    JsonRpcClient::new(transport)
}
