//! Transaction validation logic.

use super::validate::KakarotTransactionValidator;
use reth_transaction_pool::{CoinbaseTipOrdering, EthPooledTransaction, Pool, TransactionValidationTaskExecutor};

/// A type alias for the Kakarot Transaction Validator.
/// Uses the Reth implementation [`TransactionValidationTaskExecutor`].
pub type Validator<Client> =
    TransactionValidationTaskExecutor<KakarotTransactionValidator<Client, EthPooledTransaction>>;

/// A type alias for the Kakarot Transaction Ordering.
/// Uses the Reth implementation [`CoinbaseTipOrdering`].
pub type TransactionOrdering = CoinbaseTipOrdering<EthPooledTransaction>;

/// A type alias for the Kakarot Sequencer Mempool.
pub type KakarotPool<Client, S> = Pool<Validator<Client>, TransactionOrdering, S>;
