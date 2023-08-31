use reth_primitives::U256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAllowance {
    pub result: Option<U256>,
    pub error: Option<String>,
}
