use eyre::{eyre, Result};

pub struct RPCConfig {
    pub socket_addr: String,
}

impl RPCConfig {
    pub const fn new(socket_addr: String) -> Self {
        Self { socket_addr }
    }

    pub fn from_env() -> Result<Self> {
        let socket_addr = std::env::var("KAKAROT_RPC_URL")
            .map_err(|_| eyre!("Missing mandatory environment variable: KAKAROT_RPC_URL"))?;
        Ok(Self::new(socket_addr))
    }
}
