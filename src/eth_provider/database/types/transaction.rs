use reth_primitives::B256;
use reth_rpc_types::Transaction;
use serde::{Deserialize, Serialize};
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    crate::test_utils::mongo::{
        BLOCK_HASH, BLOCK_NUMBER, CHAIN_ID, EIP1599_TX_HASH, EIP2930_TX_HASH, LEGACY_TX_HASH,
        RECOVERED_EIP1599_TX_ADDRESS, RECOVERED_EIP2930_TX_ADDRESS, RECOVERED_LEGACY_TX_ADDRESS, TEST_SIG_R,
        TEST_SIG_S, TEST_SIG_V,
    },
    arbitrary::Arbitrary,
    reth_primitives::TxType,
    reth_primitives::{Address, U256},
};

/// A full transaction as stored in the database
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(any(test, feature = "arbitrary", feature = "testing"), derive(arbitrary::Arbitrary))]
pub struct StoredTransaction {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub tx: Transaction,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl StoredTransaction {
    pub fn mock_tx_with_type(tx_type: TxType) -> Self {
        match tx_type {
            TxType::Eip1559 => Self {
                tx: reth_rpc_types::Transaction {
                    hash: *EIP1599_TX_HASH,
                    block_hash: Some(*BLOCK_HASH),
                    block_number: Some(BLOCK_NUMBER),
                    transaction_index: Some(0),
                    from: *RECOVERED_EIP1599_TX_ADDRESS,
                    to: Some(Address::ZERO),
                    gas_price: Some(10),
                    gas: 100,
                    max_fee_per_gas: Some(10),
                    max_priority_fee_per_gas: Some(1),
                    signature: Some(reth_rpc_types::Signature {
                        r: *TEST_SIG_R,
                        s: *TEST_SIG_S,
                        v: *TEST_SIG_V,
                        y_parity: Some(reth_rpc_types::Parity(true)),
                    }),
                    chain_id: Some(1),
                    access_list: Some(Default::default()),
                    transaction_type: Some(tx_type.into()),
                    ..Default::default()
                },
            },
            TxType::Legacy => Self {
                tx: reth_rpc_types::Transaction {
                    hash: *LEGACY_TX_HASH,
                    block_hash: Some(*BLOCK_HASH),
                    block_number: Some(BLOCK_NUMBER),
                    transaction_index: Some(0),
                    from: *RECOVERED_LEGACY_TX_ADDRESS,
                    to: Some(Address::ZERO),
                    gas_price: Some(10),
                    gas: 100,
                    signature: Some(reth_rpc_types::Signature {
                        r: *TEST_SIG_R,
                        s: *TEST_SIG_S,
                        v: CHAIN_ID.saturating_mul(U256::from(2)).saturating_add(U256::from(35)),
                        y_parity: Default::default(),
                    }),
                    chain_id: Some(1),
                    blob_versioned_hashes: Default::default(),
                    transaction_type: Some(tx_type.into()),
                    ..Default::default()
                },
            },
            TxType::Eip2930 => Self {
                tx: reth_rpc_types::Transaction {
                    hash: *EIP2930_TX_HASH,
                    block_hash: Some(*BLOCK_HASH),
                    block_number: Some(BLOCK_NUMBER),
                    transaction_index: Some(0),
                    from: *RECOVERED_EIP2930_TX_ADDRESS,
                    to: Some(Address::ZERO),
                    gas_price: Some(10),
                    gas: 100,
                    signature: Some(reth_rpc_types::Signature {
                        r: *TEST_SIG_R,
                        s: *TEST_SIG_S,
                        v: *TEST_SIG_V,
                        y_parity: Some(reth_rpc_types::Parity(true)),
                    }),
                    chain_id: Some(1),
                    access_list: Some(Default::default()),
                    transaction_type: Some(tx_type.into()),
                    ..Default::default()
                },
            },
            TxType::Eip4844 => unimplemented!(),
        }
    }
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
                gas: u128::from(u64::arbitrary(u)?),
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
                chain_id: Some(u64::from(u32::arbitrary(u)?)),
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
    pub retries: u8,
}

impl StoredPendingTransaction {
    pub const fn new(tx: Transaction, retries: u8) -> Self {
        Self { tx, retries }
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> StoredPendingTransaction {
    pub fn arbitrary_with_optional_fields(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self { tx: StoredTransaction::arbitrary_with_optional_fields(u)?.into(), retries: u8::arbitrary(u)? })
    }
}

impl From<StoredPendingTransaction> for Transaction {
    fn from(tx: StoredPendingTransaction) -> Self {
        tx.tx
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    pub hash: B256,
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
