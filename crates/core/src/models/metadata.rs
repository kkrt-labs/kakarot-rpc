use reth_primitives::U256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub decimals: Option<U256>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub error: Option<String>,
}
