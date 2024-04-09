use eyre::{eyre, Result};

/// Represents the configuration for RPC.
#[derive(Debug)]
pub struct RPCConfig {
    /// The socket address for the RPC.
    pub socket_addr: String,
}

impl RPCConfig {
    /// Creates a new RPC configuration with the given socket address.
    pub const fn new(socket_addr: String) -> Self {
        Self { socket_addr }
    }

    /// Creates an RPC configuration by reading the socket address from the environment variable `KAKAROT_RPC_URL`.
    pub fn from_env() -> Result<Self> {
        let socket_addr = std::env::var("KAKAROT_RPC_URL")
            .map_err(|_| eyre!("Missing mandatory environment variable: KAKAROT_RPC_URL"))?;
        Ok(Self::new(socket_addr))
    }

    /// Creates an RPC configuration with the provided port, by replacing the port in the existing socket address.
    pub fn from_port(port: u16) -> Result<Self> {
        let mut config = Self::from_env()?;
        // Remove port from socket address and replace it with provided port
        let parts: Vec<&str> = config.socket_addr.split(':').collect();
        if let Some(addr) = parts.first() {
            config.socket_addr = format!("{}:{}", addr, port);
        }
        Ok(config)
    }

    /// Creates a new RPC configuration specifically for testing environments.
    #[cfg(feature = "testing")]
    pub fn new_test_config() -> Self {
        // Hardcode the socket address for testing environment
        Self::new("127.0.0.1:3030".to_string())
    }

    /// Creates a new RPC configuration for testing environments with the provided port.
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
