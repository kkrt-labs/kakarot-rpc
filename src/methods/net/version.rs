use jsonrpc_http_server::{jsonrpc_core::Error, jsonrpc_core::Params, jsonrpc_core::Value};

use crate::utils::constants::CHAINID;

pub const METHOD: crate::methods::Method = crate::methods::Method {
    prefix: "net",
    name: "version",
};

pub async fn execute(_params: Params) -> Result<Value, Error> {
    return Ok(Value::String(CHAINID.to_string()));
}
