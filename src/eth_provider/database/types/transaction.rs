use reth_primitives::B256;
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use reth_primitives::{Address, TransactionSigned, U256, U64, U8};
use reth_rpc_types::Transaction;
use serde::{Deserialize, Serialize};

/// A full transaction as stored in the database
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct StoredTransaction {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub tx: Transaction,
}

impl From<StoredTransaction> for Transaction {
    fn from(tx: StoredTransaction) -> Self {
        tx.tx
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> arbitrary::Arbitrary<'a> for StoredTransaction {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let transaction = TransactionSigned::arbitrary(u)?;

        Ok(StoredTransaction {
            tx: Transaction {
                hash: transaction.hash,
                nonce: transaction.nonce(),
                block_hash: Some(B256::arbitrary(u)?),
                block_number: Some(U256::from(U64::arbitrary(u)?)),
                transaction_index: Some(U256::from(U64::arbitrary(u)?)),
                from: Address::arbitrary(u)?,
                to: transaction.to(),
                value: transaction.value(),
                gas_price: Some(U256::from(U128::arbitrary(u)?)),
                gas: U256::from(U64::arbitrary(u)?),
                max_fee_per_gas: Some(U256::from(transaction.max_fee_per_gas())),
                max_priority_fee_per_gas: Some(U256::from(transaction.max_priority_fee_per_gas().unwrap_or_default())),
                max_fee_per_blob_gas: transaction.max_fee_per_blob_gas().map(U256::from),
                input: transaction.input().clone(),
                signature: Some(reth_rpc_types::Signature {
                    r: transaction.signature.r,
                    s: transaction.signature.s,
                    v: U256::arbitrary(u)?,
                    y_parity: Some(reth_rpc_types::Parity(bool::arbitrary(u)?)),
                }),
                chain_id: transaction.chain_id(),
                blob_versioned_hashes: transaction.blob_versioned_hashes(),
                access_list: transaction.access_list().map(|list| {
                    reth_rpc_types::AccessList(
                        list.0
                            .iter()
                            .map(|item| reth_rpc_types::AccessListItem {
                                address: item.address,
                                storage_keys: item.storage_keys.clone(),
                            })
                            .collect(),
                    )
                }),

                transaction_type: Some(U8::from(Into::<u8>::into(transaction.tx_type()) % 3)),
                other: Default::default(),
            },
        })
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
