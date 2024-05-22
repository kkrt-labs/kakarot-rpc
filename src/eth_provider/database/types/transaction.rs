use reth_primitives::B256;
use reth_rpc_types::Transaction;
use serde::{Deserialize, Serialize};
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {arbitrary::Arbitrary, reth_primitives::TxType};

/// A full transaction as stored in the database
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(any(test, feature = "arbitrary", feature = "testing"), derive(arbitrary::Arbitrary))]
pub struct StoredTransaction {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub tx: Transaction,
}

impl From<StoredTransaction> for Transaction {
    fn from(tx: StoredTransaction) -> Self {
        tx.tx
    }
}

impl From<Transaction> for StoredTransaction {
    fn from(tx: Transaction) -> Self {
        Self { tx }
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> StoredTransaction {
    pub fn arbitrary_with_optional_fields(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let transaction = Transaction::arbitrary(u)?;

        let transaction_type = Into::<u8>::into(transaction.transaction_type.unwrap_or_default()) % 3;

        Ok(Self {
            tx: Transaction {
                block_hash: Some(B256::arbitrary(u)?),
                block_number: Some(u64::arbitrary(u)?),
                transaction_index: Some(u64::arbitrary(u)?),
                gas_price: Some(u128::arbitrary(u)?),
                gas: u64::arbitrary(u)? as u128,
                max_fee_per_gas: if TryInto::<TxType>::try_into(transaction_type).unwrap() == TxType::Legacy {
                    None
                } else {
                    Some(u128::arbitrary(u)?)
                },
                max_priority_fee_per_gas: if TryInto::<TxType>::try_into(transaction_type).unwrap() == TxType::Legacy {
                    None
                } else {
                    Some(u128::arbitrary(u)?)
                },
                signature: Some(reth_rpc_types::Signature {
                    y_parity: Some(reth_rpc_types::Parity(bool::arbitrary(u)?)),
                    ..reth_rpc_types::Signature::arbitrary(u)?
                }),
                transaction_type: Some(transaction_type),
                chain_id: Some(u32::arbitrary(u)? as u64),
                other: Default::default(),
                access_list: Some(reth_rpc_types::AccessList::arbitrary(u)?),
                ..transaction
            },
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct StoredPendingTransaction {
    /// Transaction object
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub tx: Transaction,
    /// Number of retries
    pub retries: u64,
}

impl StoredPendingTransaction {
    pub const fn new(tx: Transaction, retries: u64) -> Self {
        Self { tx, retries }
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> StoredPendingTransaction {
    pub fn arbitrary_with_optional_fields(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self { tx: StoredTransaction::arbitrary_with_optional_fields(u)?.into(), retries: u64::arbitrary(u)? })
    }
}

impl From<Transaction> for StoredPendingTransaction {
    fn from(tx: Transaction) -> Self {
        Self { tx, retries: 0 }
    }
}

/// A transaction hash as stored in the database
/// This wrapper is used to deserialize a transaction
/// from the database, on which a projection was
/// performed in order to only return the transaction
/// hash (e.g. {tx: {hash: "0x1234"}})
#[derive(Debug, Deserialize)]
pub struct StoredTransactionHash {
    #[serde(rename = "tx")]
    pub tx_hash: Hash,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    pub hash: B256,
}

impl From<StoredTransactionHash> for B256 {
    fn from(hash: StoredTransactionHash) -> Self {
        hash.tx_hash.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn test_stored_transaction_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _ = StoredTransaction::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }
}
