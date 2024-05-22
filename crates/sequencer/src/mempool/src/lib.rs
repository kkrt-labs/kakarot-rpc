//! The Kakarot mempool implementation.
//!
//! ## Overview
//!
//! The mempool crate provides the core logic for managing transactions in the mempool.
//!
//! ## Implementation
//!
//! The Kakarot mempool implementation reuses where possible components from the Reth
//! [mempool implementation](https://github.com/paradigmxyz/reth/tree/main/crates/transaction-pool/src).

pub mod validate;

use reth_transaction_pool::{
    CoinbaseTipOrdering, EthPooledTransaction, EthTransactionValidator, Pool, TransactionValidationTaskExecutor,
};

/// A type alias for the Kakarot Sequencer Mempool.
pub type KakarotMempool<Client, S> = Pool<
    TransactionValidationTaskExecutor<EthTransactionValidator<Client, EthPooledTransaction>>,
    CoinbaseTipOrdering<EthPooledTransaction>,
    S,
>;
