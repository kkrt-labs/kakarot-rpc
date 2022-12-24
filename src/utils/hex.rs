use hex::FromHex;
use jsonrpc_http_server::jsonrpc_core::Error;

use crate::utils::error::default_error_invalid_params;

pub fn parse_hex_bytes(hex: &String) -> Result<Vec<u8>, Error> {
    if hex.len() < 3 || &hex[0..2] != "0x" || hex.len() % 2 == 1 {
        return Err(default_error_invalid_params(Some(
            "Hex must start with 0x, have an even length, and have more than 1 digits",
        )));
    }
    match Vec::from_hex(&hex[2..]) {
        Ok(v) => Ok(v),
        Err(_) => Err(default_error_invalid_params(None)),
    }
}

pub fn parse_hex_quantity<T: FromHex>(hex: &String) -> Result<T, Error> {
    if hex.len() < 3 || &hex[0..2] != "0x" {
        return Err(default_error_invalid_params(Some(
            "Hex must start with 0x, and have more than 1 digits",
        )));
    }
    let mut _hex = hex[2..].to_string();
    if hex.len() % 2 == 1 {
        _hex = "0".to_string() + &_hex;
    }
    match T::from_hex(&_hex) {
        Ok(v) => Ok(v),
        Err(_) => Err(default_error_invalid_params(None)),
    }
}

pub fn format_hex(hex: &String) -> String {
    return "0x".to_string() + hex;
}
