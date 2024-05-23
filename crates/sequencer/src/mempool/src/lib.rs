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

/// A type alias for the Kakarot Transaction Validator.
/// Uses the Reth implementation [TransactionValidationTaskExecutor].
pub type Validator<Client> = TransactionValidationTaskExecutor<EthTransactionValidator<Client, EthPooledTransaction>>;

/// A type alias for the Kakarot Transaction Ordering.
/// Uses the Reth implementation [CoinbaseTipOrdering].
pub type TransactionOrdering = CoinbaseTipOrdering<EthPooledTransaction>;

/// A type alias for the Kakarot Sequencer Mempool.
pub type Mempool<Client, S> = Pool<Validator<Client>, TransactionOrdering, S>;
