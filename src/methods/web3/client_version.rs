use jsonrpc_http_server::{jsonrpc_core::Error, jsonrpc_core::Params, jsonrpc_core::Value};

pub const METHOD: crate::methods::Method = crate::methods::Method {
    prefix: "web3",
    name: "clientVersion",
};

pub async fn execute(_params: Params) -> Result<Value, Error> {
    return Ok(Value::String(
        env!("CARGO_PKG_NAME").to_string() + "/v" + env!("CARGO_PKG_VERSION"),
    ));
}
