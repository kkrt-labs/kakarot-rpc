use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use starknet::core::types::{MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    pub contract_address: Address,
    pub token_balance: Option<U256>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    pub address: Address,
    pub token_balances: Vec<TokenBalance>,
}

pub struct BlockWithTxHashes(MaybePendingBlockWithTxHashes);

pub struct BlockWithTxs(MaybePendingBlockWithTxs);
