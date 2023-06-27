use eyre::{eyre, Result};

pub struct RPCConfig {
    pub socket_addr: String,
}

impl RPCConfig {
    pub fn new(socket_addr: String) -> RPCConfig {
        RPCConfig { socket_addr }
    }

    pub fn from_env() -> Result<Self> {
        let socket_addr = std::env::var("KAKAROT_HTTP_RPC_ADDRESS")
            .map_err(|_| eyre!("Missing mandatory environment variable: KAKAROT_HTTP_RPC_ADDRESS"))?;
        Ok(RPCConfig::new(socket_addr))
    }
}
