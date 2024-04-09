#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use reth_primitives::{Address, Bloom, Receipt, B256, U128, U256, U64, U8};
use reth_rpc_types::TransactionReceipt;
use serde::{Deserialize, Serialize};

/// A transaction receipt as stored in the database
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct StoredTransactionReceipt {
    /// The transaction receipt.
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub receipt: TransactionReceipt,
}

impl From<StoredTransactionReceipt> for TransactionReceipt {
    fn from(receipt: StoredTransactionReceipt) -> Self {
        receipt.receipt
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> arbitrary::Arbitrary<'a> for StoredTransactionReceipt {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let receipt = Receipt::arbitrary(u)?;

        let mut rpc_logs = Vec::new();

        for log in receipt.logs {
            rpc_logs.push(reth_rpc_types::Log {
                address: log.address,
                topics: log.topics,
                data: log.data,
                block_hash: Some(B256::arbitrary(u)?),
                block_number: Some(U256::arbitrary(u)?),
                transaction_hash: Some(B256::arbitrary(u)?),
                transaction_index: Some(U256::arbitrary(u)?),
                log_index: Some(U256::arbitrary(u)?),
                removed: bool::arbitrary(u)?,
            })
        }

        Ok(Self {
            receipt: TransactionReceipt {
                transaction_hash: Some(B256::arbitrary(u)?),
                transaction_index: U64::arbitrary(u)?,
                block_hash: Some(B256::arbitrary(u)?),
                block_number: Some(U256::arbitrary(u)?),
                cumulative_gas_used: U256::from(receipt.cumulative_gas_used),
                gas_used: Some(U256::arbitrary(u)?),
                effective_gas_price: U128::arbitrary(u)?,
                blob_gas_used: Some(U128::arbitrary(u)?),
                blob_gas_price: Some(U128::arbitrary(u)?),
                from: Address::arbitrary(u)?,
                to: Some(Address::arbitrary(u)?),
                contract_address: Some(Address::arbitrary(u)?),
                logs: rpc_logs,
                logs_bloom: Bloom::arbitrary(u)?,
                state_root: Some(B256::arbitrary(u)?),
                status_code: Some(U64::from(receipt.success as u8)),
                transaction_type: U8::from::<u8>(Into::<u8>::into(receipt.tx_type) % 3),
                other: Default::default(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn test_stored_transaction_receipt_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _ = StoredTransactionReceipt::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }
}
