use crate::utils::error::default_error_invalid_params;
use crate::utils::hex::parse_hex_bytes;
use crate::{crypto::sha3::Sha3, utils::hex::format_hex};
use crypto::digest::Digest;
use jsonrpc_http_server::jsonrpc_core::{Error, Params, Value};

pub const METHOD: crate::methods::Method = crate::methods::Method {
    prefix: "web3",
    name: "sha3",
};

pub async fn execute(_params: Params) -> Result<Value, Error> {
    match _params.parse::<Vec<String>>() {
        Ok(data) => {
            if data.len() < 1 {
                return Err(default_error_invalid_params());
            }
            let mut hasher = Sha3::keccak256(); // image of an hasher https://yt3.ggpht.com/icJZDespcjNLPi-_1qA-_kYIfWq66_mJM-721fhpA1f-yZ6st5-Wooqn0MS9TQXj8jTbYNVpoQ=s176-c-k-c0x00ffffff-no-rj
            match parse_hex_bytes(&data[0]) {
                Ok(hex) => {
                    hasher.input(&hex);
                    return Ok(Value::String(format_hex(&hasher.result_str())));
                }
                Err(e) => Err(e),
            }
        }
        Err(_) => Err(default_error_invalid_params()),
    }
}
