use eyre::{eyre, Result};

#[derive(Debug)]
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

    pub fn from_port(port: u16) -> Result<Self> {
        let mut config = Self::from_env()?;
        // Remove port from socket address and replace it with provided port
        let parts: Vec<&str> = config.socket_addr.split(':').collect();
        if let Some(addr) = parts.first() {
            config.socket_addr = format!("{}:{}", addr, port);
        }
        Ok(config)
    }

    #[cfg(feature = "testing")]
    pub fn new_test_config() -> Self {
        // Hardcode the socket address for testing environment
        Self::new("127.0.0.1:3030".to_string())
    }

    #[cfg(feature = "testing")]
    pub fn new_test_config_from_port(port: u16) -> Self {
        let mut config = Self::new_test_config();
        // Remove port from socket address and replace it with provided port
        let parts: Vec<&str> = config.socket_addr.split(':').collect();
        if let Some(addr) = parts.first() {
            config.socket_addr = format!("{}:{}", addr, port);
        }
        config
    }
}
