use alloy_rpc_types::TransactionReceipt;
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use reth_primitives::Receipt;
use serde::{Deserialize, Serialize};

/// A transaction receipt as stored in the database
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct StoredTransactionReceipt {
    #[serde(deserialize_with = "crate::providers::eth_provider::database::types::serde::deserialize_intermediate")]
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
        use alloy_primitives::{Address, Bloom, B256};

        let receipt = Receipt::arbitrary(u)?;

        let mut logs = Vec::new();

        for log in receipt.logs {
            logs.push(alloy_rpc_types::Log {
                transaction_index: Some(u64::arbitrary(u)?),
                log_index: Some(u64::arbitrary(u)?),
                removed: bool::arbitrary(u)?,
                inner: log,
                block_hash: Some(B256::arbitrary(u)?),
                block_number: Some(u64::arbitrary(u)?),
                block_timestamp: Some(u64::arbitrary(u)?),
                transaction_hash: Some(B256::arbitrary(u)?),
            });
        }

        let receipt = alloy_rpc_types::ReceiptWithBloom {
            receipt: alloy_rpc_types::Receipt {
                status: bool::arbitrary(u)?.into(),
                cumulative_gas_used: u128::from(u64::arbitrary(u)?),
                logs,
            },
            logs_bloom: Bloom::arbitrary(u)?,
        };

        Ok(Self {
            receipt: TransactionReceipt {
                transaction_hash: B256::arbitrary(u)?,
                transaction_index: Some(u64::arbitrary(u)?),
                block_hash: Some(B256::arbitrary(u)?),
                block_number: Some(u64::arbitrary(u)?),
                gas_used: u128::arbitrary(u)?,
                effective_gas_price: u128::arbitrary(u)?,
                blob_gas_used: Some(u128::arbitrary(u)?),
                blob_gas_price: Some(u128::arbitrary(u)?),
                from: Address::arbitrary(u)?,
                to: Some(Address::arbitrary(u)?),
                contract_address: Some(Address::arbitrary(u)?),
                state_root: Some(B256::arbitrary(u)?),
                inner: match u.int_in_range(0..=3)? {
                    0 => alloy_consensus::ReceiptEnvelope::Legacy(receipt),
                    1 => alloy_consensus::ReceiptEnvelope::Eip2930(receipt),
                    2 => alloy_consensus::ReceiptEnvelope::Eip1559(receipt),
                    3 => alloy_consensus::ReceiptEnvelope::Eip4844(receipt),
                    _ => unreachable!(),
                },
                authorization_list: None,
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
