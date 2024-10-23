use alloy_primitives::B256;
use alloy_rpc_types::Transaction;
use alloy_serde::WithOtherFields;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    alloy_primitives::U256,
    arbitrary::Arbitrary,
    rand::Rng,
    reth_primitives::transaction::legacy_parity,
    reth_testing_utils::generators::{self},
};
// This a type alias that is defined to simplify its usages and management through the basecode
pub type ExtendedTransaction = WithOtherFields<Transaction>;

/// A full transaction as stored in the database
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct StoredTransaction {
    #[serde(deserialize_with = "crate::providers::eth_provider::database::types::serde::deserialize_intermediate")]
    pub tx: WithOtherFields<Transaction>,
}

impl From<StoredTransaction> for WithOtherFields<Transaction> {
    fn from(tx: StoredTransaction) -> Self {
        tx.tx
    }
}

impl From<&StoredTransaction> for WithOtherFields<Transaction> {
    fn from(tx: &StoredTransaction) -> Self {
        tx.tx.clone()
    }
}

impl From<WithOtherFields<Transaction>> for StoredTransaction {
    fn from(tx: WithOtherFields<Transaction>) -> Self {
        Self { tx }
    }
}

impl Deref for StoredTransaction {
    type Target = WithOtherFields<Transaction>;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl Arbitrary<'_> for StoredTransaction {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        // Initialize a random number generator.
        let mut rng = generators::rng();
        // Generate a random integer between 0 and 2 to decide which transaction type to create.
        let random_choice = rng.gen_range(0..3);

        // Create a `primitive_tx` of a specific transaction type based on the random choice.
        let primitive_tx = match random_choice {
            0 => reth_primitives::Transaction::Legacy(alloy_consensus::TxLegacy {
                chain_id: Some(u8::arbitrary(u)?.into()),
                ..Arbitrary::arbitrary(u)?
            }),
            1 => reth_primitives::Transaction::Eip2930(alloy_consensus::TxEip2930::arbitrary(u)?),
            _ => reth_primitives::Transaction::Eip1559(alloy_consensus::TxEip1559::arbitrary(u)?),
        };

        // Sign the generated transaction with a randomly generated key pair.
        let transaction_signed = generators::sign_tx_with_random_key_pair(&mut rng, primitive_tx);

        // Initialize a `Transaction` structure and populate it with the signed transaction's data.
        let mut tx = Transaction {
            hash: transaction_signed.hash,
            from: transaction_signed.recover_signer().unwrap(),
            block_hash: Some(B256::arbitrary(u)?),
            block_number: Some(u64::arbitrary(u)?),
            transaction_index: Some(u64::arbitrary(u)?),
            signature: Some(alloy_rpc_types::Signature {
                r: transaction_signed.signature.r(),
                s: transaction_signed.signature.s(),
                v: if transaction_signed.is_legacy() {
                    U256::from(legacy_parity(&transaction_signed.signature, transaction_signed.chain_id()).to_u64())
                } else {
                    U256::from(transaction_signed.signature.v().to_u64())
                },
                y_parity: Some((transaction_signed.signature.v().y_parity()).into()),
            }),
            nonce: transaction_signed.nonce(),
            value: transaction_signed.value(),
            input: transaction_signed.input().clone(),
            chain_id: transaction_signed.chain_id(),
            transaction_type: Some(transaction_signed.tx_type().into()),
            to: transaction_signed.to(),
            gas: transaction_signed.gas_limit(),
            ..Default::default()
        };

        // Populate the `tx` structure based on the specific type of transaction.
        match transaction_signed.transaction {
            reth_primitives::Transaction::Legacy(transaction) => {
                tx.gas_price = Some(transaction.gas_price);
            }
            reth_primitives::Transaction::Eip2930(transaction) => {
                tx.access_list = Some(transaction.access_list);
                tx.gas_price = Some(transaction.gas_price);
            }
            reth_primitives::Transaction::Eip1559(transaction) => {
                tx.max_fee_per_gas = Some(transaction.max_fee_per_gas);
                tx.max_priority_fee_per_gas = Some(transaction.max_priority_fee_per_gas);
                tx.access_list = Some(transaction.access_list);
            }
            reth_primitives::Transaction::Eip4844(_) | reth_primitives::Transaction::Eip7702(_) => {
                unreachable!("Non supported transaction type")
            }
        };

        // Return the constructed `StoredTransaction` instance.
        Ok(Self { tx: WithOtherFields::new(tx) })
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

    #[test]
    fn random_tx_signature() {
        for _ in 0..10 {
            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());

            // Generate a random transaction
            let transaction = StoredTransaction::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();

            // Extract the signature from the generated transaction.
            let signature = transaction.signature.unwrap().try_into().unwrap();

            // Convert the transaction to primitive type.
            let tx = transaction.clone().tx.try_into().unwrap();

            // Reconstruct the signed transaction using the extracted `tx` and `signature`.
            let transaction_signed = reth_primitives::TransactionSigned::from_transaction_and_signature(tx, signature);

            // Verify that the `from` address in the original transaction matches the recovered signer address
            // from the reconstructed signed transaction. This confirms that the signature is valid.
            assert_eq!(transaction.from, transaction_signed.recover_signer().unwrap());
        }
    }
}
