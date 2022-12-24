use hex::FromHex;
use jsonrpc_http_server::jsonrpc_core::Error;

use crate::utils::error::default_error_invalid_params;

pub fn parse_hex(hex: &String) -> Result<Vec<u8>, Error> {
    if &hex[0..2] != "0x" {
        return Err(default_error_invalid_params());
    }
    match Vec::from_hex(&hex[2..]) {
        // add padding 0 if odd length
        Ok(v) => Ok(v),
        Err(_) => Err(default_error_invalid_params()),
    }
}

pub fn format_hex(hex: &String) -> String {
    return "0x".to_string() + hex;
}
