use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// Represents the balance of a specific ERC20 token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    /// The address of the ERC20 token.
    pub token_address: Address,
    /// The balance of the ERC20 token.
    pub token_balance: U256,
}

/// Represents the balances of multiple ERC20 tokens for a specific address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    /// The address for which the token balances are queried.
    pub address: Address,
    /// A list of token balances associated with the address.
    pub token_balances: Vec<TokenBalance>,
}

/// Represents the metadata (decimals, name, symbol) of an ERC20 token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// The number of decimals the token uses.
    pub decimals: U256,
    /// The name of the token.
    pub name: String,
    /// The symbol of the token.
    pub symbol: String,
}
