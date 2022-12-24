use jsonrpc_http_server::jsonrpc_core::{Error, ErrorCode, Value};

pub fn default_error_invalid_params(msg: Option<&str>) -> Error {
    let mut message = "Invalid Params".to_string();
    if let Some(m) = msg {
        message += &(": ".to_owned() + &m);
    }
    return Error {
        code: ErrorCode::InvalidParams,
        message,
        data: Some(Value::Null),
    };
}

pub fn default_error_internal_error(msg: Option<&str>) -> Error {
    let mut message = "Internal error".to_string();
    if let Some(m) = msg {
        message += &(": ".to_owned() + &m);
    }
    return Error {
        code: ErrorCode::InternalError,
        message,
        data: Some(Value::Null),
    };
}
