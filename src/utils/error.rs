use jsonrpc_http_server::jsonrpc_core::{Error, ErrorCode, Value};

pub fn default_error_invalid_params() -> Error {
    return Error {
        code: ErrorCode::InvalidParams,
        message: "Invalid Params".to_string(),
        data: Some(Value::Null),
    };
}
