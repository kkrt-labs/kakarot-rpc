#![allow(unused_variables, clippy::struct_excessive_bools)]

use crate::providers::eth_provider::{
    database::state::EthDatabase, provider::EthereumProvider,
    starknet::kakarot_core::get_white_listed_eip_155_transaction_hashes,
};
use alloy_rpc_types::BlockNumberOrTag;
use reth_chainspec::ChainSpec;
use reth_primitives::{
    GotExpected, InvalidTransactionError, SealedBlock, EIP1559_TX_TYPE_ID, EIP2930_TX_TYPE_ID, EIP4844_TX_TYPE_ID,
    LEGACY_TX_TYPE_ID,
};
use reth_revm::DatabaseRef;
use reth_transaction_pool::{
    error::InvalidPoolTransactionError,
    validate::{ensure_intrinsic_gas, ForkTracker, ValidTransaction, DEFAULT_MAX_TX_INPUT_BYTES},
    EthPoolTransaction, TransactionOrigin, TransactionValidationOutcome, TransactionValidator,
};
use std::{
    marker::PhantomData,
    sync::{atomic::AtomicBool, Arc},
};

#[derive(Debug, Clone)]
pub struct KakarotTransactionValidatorBuilder {
    pub chain_spec: Arc<ChainSpec>,
    /// Fork indicator whether we are in the Shanghai stage.
    pub shanghai: bool,
    /// Fork indicator whether we are in the Cancun hardfork.
    pub cancun: bool,
    /// Fork indicator whether we are in the Prague hardfork.
    pub prague: bool,
    /// Whether using EIP-2718 type transactions is allowed
    pub eip2718: bool,
    /// Whether using EIP-1559 type transactions is allowed
    pub eip1559: bool,
    /// Whether using EIP-4844 type transactions is allowed
    pub eip4844: bool,
    /// The current max gas limit
    pub block_gas_limit: u64,
    /// Max size in bytes of a single transaction allowed
    pub max_tx_input_bytes: usize,
}

impl KakarotTransactionValidatorBuilder {
    /// Creates a new builder for the given [`ChainSpec`]
    ///
    /// By default, this assumes the network is on the `Cancun` hardfork and the following
    /// transactions are allowed:
    ///  - Legacy
    ///  - EIP-2718
    ///  - EIP-1559
    pub fn new(chain_spec: &Arc<ChainSpec>) -> Self {
        Self {
            chain_spec: chain_spec.clone(),
            block_gas_limit: chain_spec.max_gas_limit,
            max_tx_input_bytes: DEFAULT_MAX_TX_INPUT_BYTES,

            // by default all transaction types are allowed except EIP-4844
            eip2718: true,
            eip1559: true,
            eip4844: false,

            // shanghai is activated by default
            shanghai: true,

            // cancun is activated by default
            cancun: true,

            // prague not yet activated
            prague: false,
        }
    }

    /// Builds the [`EthTransactionValidator`] without spawning validator tasks.
    pub fn build<P, Tx>(self, provider: P) -> KakarotTransactionValidator<P, Tx>
    where
        P: EthereumProvider + Send + Sync,
    {
        let Self {
            chain_spec,
            shanghai,
            cancun,
            prague,
            eip2718,
            eip1559,
            eip4844,
            block_gas_limit,
            max_tx_input_bytes,
            ..
        } = self;

        let fork_tracker = ForkTracker {
            shanghai: AtomicBool::new(shanghai),
            cancun: AtomicBool::new(cancun),
            prague: AtomicBool::new(prague),
        };

        let inner = KakarotTransactionValidatorInner {
            chain_spec,
            provider,
            eip2718,
            eip1559,
            eip4844,
            block_gas_limit,
            max_tx_input_bytes,
            fork_tracker,
            _marker: Default::default(),
        };

        KakarotTransactionValidator { inner: Arc::new(inner) }
    }
}

/// Validator for Ethereum transactions.
#[derive(Debug, Clone)]
pub struct KakarotTransactionValidator<P, T>
where
    P: EthereumProvider + Send + Sync,
{
    /// The type that performs the actual validation.
    inner: Arc<KakarotTransactionValidatorInner<P, T>>,
}

impl<P, Tx> KakarotTransactionValidator<P, Tx>
where
    P: EthereumProvider + Send + Sync,
{
    /// Returns the configured chain spec
    pub fn chain_spec(&self) -> Arc<ChainSpec> {
        self.inner.chain_spec.clone()
    }

    /// Returns the provider
    pub fn provider(&self) -> &P {
        &self.inner.provider
    }
}

impl<P, Tx> KakarotTransactionValidator<P, Tx>
where
    P: EthereumProvider + Send + Sync,
    Tx: EthPoolTransaction,
{
    /// Validates a single transaction.
    ///
    /// See also [`TransactionValidator::validate_transaction`]
    pub fn validate_one(&self, transaction: Tx) -> TransactionValidationOutcome<Tx> {
        self.inner.validate_one(transaction)
    }

    /// Validates all given transactions.
    ///
    /// Returns all outcomes for the given transactions in the same order.
    ///
    /// See also [`Self::validate_one`]
    pub fn validate_all(&self, transactions: Vec<(TransactionOrigin, Tx)>) -> Vec<TransactionValidationOutcome<Tx>> {
        transactions.into_iter().map(|(_origin, tx)| self.validate_one(tx)).collect()
    }
}

impl<P, Tx> TransactionValidator for KakarotTransactionValidator<P, Tx>
where
    P: EthereumProvider + Send + Sync,
    Tx: EthPoolTransaction,
{
    type Transaction = Tx;

    async fn validate_transaction(
        &self,
        _origin: TransactionOrigin,
        transaction: Self::Transaction,
    ) -> TransactionValidationOutcome<Self::Transaction> {
        self.validate_one(transaction)
    }

    async fn validate_transactions(
        &self,
        transactions: Vec<(TransactionOrigin, Self::Transaction)>,
    ) -> Vec<TransactionValidationOutcome<Self::Transaction>> {
        self.validate_all(transactions)
    }

    fn on_new_head_block(&self, _new_tip_block: &SealedBlock) {}
}

/// A [`TransactionValidator`] implementation that validates ethereum transaction.
#[derive(Debug)]
pub(crate) struct KakarotTransactionValidatorInner<P, T>
where
    P: EthereumProvider + Send + Sync,
{
    /// Spec of the chain
    chain_spec: Arc<ChainSpec>,
    /// This type fetches network info.
    provider: P,
    /// Fork indicator whether we are using EIP-2718 type transactions.
    eip2718: bool,
    /// Fork indicator whether we are using EIP-1559 type transactions.
    eip1559: bool,
    /// Fork indicator whether we are using EIP-4844 blob transactions.
    eip4844: bool,
    /// The current max gas limit
    block_gas_limit: u64,
    /// Maximum size in bytes a single transaction can have in order to be accepted into the pool.
    max_tx_input_bytes: usize,
    /// tracks activated forks relevant for transaction validation
    fork_tracker: ForkTracker,
    /// Marker for the transaction type
    _marker: PhantomData<T>,
}

impl<P, Tx> KakarotTransactionValidatorInner<P, Tx>
where
    P: EthereumProvider + Send + Sync,
{
    /// Returns the configured chain id
    pub(crate) fn chain_id(&self) -> u64 {
        self.chain_spec.chain().id()
    }
}

impl<P, Tx> KakarotTransactionValidatorInner<P, Tx>
where
    P: EthereumProvider + Send + Sync,
    Tx: EthPoolTransaction,
{
    /// Validates a single transaction.
    #[allow(clippy::too_many_lines)]
    fn validate_one(&self, transaction: Tx) -> TransactionValidationOutcome<Tx> {
        // Checks for tx_type
        match transaction.tx_type() {
            LEGACY_TX_TYPE_ID => {
                if transaction.chain_id().is_none()
                    && !get_white_listed_eip_155_transaction_hashes().contains(transaction.hash())
                {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::TxTypeNotSupported.into(),
                    );
                }
            }
            EIP2930_TX_TYPE_ID => {
                // Accept only legacy transactions until EIP-2718/2930 activates
                if !self.eip2718 {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::Eip2930Disabled.into(),
                    );
                }
            }
            EIP1559_TX_TYPE_ID => {
                // Reject dynamic fee transactions until EIP-1559 activates.
                if !self.eip1559 {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::Eip1559Disabled.into(),
                    );
                }
            }
            EIP4844_TX_TYPE_ID => {
                // Reject blob transactions.
                if !self.eip4844 {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::Eip4844Disabled.into(),
                    );
                }
            }
            _ => {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidTransactionError::TxTypeNotSupported.into(),
                )
            }
        };

        // Reject transactions over defined size to prevent DOS attacks
        let transaction_size = transaction.size();
        if transaction_size > self.max_tx_input_bytes {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidPoolTransactionError::OversizedData(transaction_size, self.max_tx_input_bytes),
            );
        }

        // Checks for gas limit
        let transaction_gas_limit = transaction.gas_limit();
        if transaction_gas_limit > self.block_gas_limit {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidPoolTransactionError::ExceedsGasLimit(transaction_gas_limit, self.block_gas_limit),
            );
        }

        // Ensure max_priority_fee_per_gas (if EIP1559) is less than max_fee_per_gas if any.
        if transaction.max_priority_fee_per_gas() > Some(transaction.max_fee_per_gas()) {
            return TransactionValidationOutcome::Invalid(transaction, InvalidTransactionError::TipAboveFeeCap.into());
        }

        // Checks for chainid
        if let Some(chain_id) = transaction.chain_id() {
            if chain_id != self.chain_id() {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidTransactionError::ChainIdMismatch.into(),
                );
            }
        }

        // intrinsic gas checks
        if let Err(err) = ensure_intrinsic_gas(&transaction, &self.fork_tracker) {
            return TransactionValidationOutcome::Invalid(transaction, err);
        }

        // Fetch the account state for the Pending block
        let db = EthDatabase::new(Arc::new(&self.provider), BlockNumberOrTag::Pending.into());
        let account = match db.basic_ref(transaction.sender()) {
            Ok(account) => account.unwrap_or_default(),
            Err(err) => return TransactionValidationOutcome::Error(*transaction.hash(), Box::new(err)),
        };

        // Signer account shouldn't have bytecode. Presence of bytecode means this is a
        // smartcontract.
        if !account.is_empty_code_hash() {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidTransactionError::SignerAccountHasBytecode.into(),
            );
        }

        // Checks for nonce
        if transaction.nonce() < account.nonce {
            return TransactionValidationOutcome::Invalid(
                transaction.clone(),
                InvalidTransactionError::NonceNotConsistent { tx: transaction.nonce(), state: account.nonce }.into(),
            );
        }

        let cost = transaction.cost();

        // Checks for max cost
        if cost > account.balance {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidTransactionError::InsufficientFunds(GotExpected { got: account.balance, expected: cost }.into())
                    .into(),
            );
        }

        let maybe_blob_sidecar = None;

        // Return the valid transaction
        TransactionValidationOutcome::Valid {
            balance: account.balance,
            state_nonce: account.nonce,
            transaction: ValidTransaction::new(transaction, maybe_blob_sidecar),
            // by this point assume all external transactions should be propagated
            propagate: true,
        }
    }
}
