use reth_primitives::B256;
use reth_rpc_types::Transaction;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    alloy_signer::SignerSync,
    alloy_signer_local::PrivateKeySigner,
    arbitrary::Arbitrary,
    reth_primitives::{TransactionSignedNoHash, U256},
};

/// A full transaction as stored in the database
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(any(test, feature = "arbitrary", feature = "testing"), derive(arbitrary::Arbitrary))]
pub struct StoredTransaction {
    #[serde(deserialize_with = "crate::providers::eth_provider::database::types::serde::deserialize_intermediate")]
    pub tx: Transaction,
}

impl From<StoredTransaction> for Transaction {
    fn from(tx: StoredTransaction) -> Self {
        tx.tx
    }
}

impl From<&StoredTransaction> for Transaction {
    fn from(tx: &StoredTransaction) -> Self {
        tx.tx.clone()
    }
}

impl From<Transaction> for StoredTransaction {
    fn from(tx: Transaction) -> Self {
        Self { tx }
    }
}

impl Deref for StoredTransaction {
    type Target = Transaction;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> StoredTransaction {
    pub fn arbitrary_with_optional_fields(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut primitive_transaction = reth_primitives::Transaction::arbitrary(u)?;

        // Ensure the transaction is not a blob transaction
        while primitive_transaction.tx_type() == 3 {
            primitive_transaction = reth_primitives::Transaction::arbitrary(u)?;
        }

        // Force the chain ID to be set
        let chain_id = u32::arbitrary(u)?.into();
        primitive_transaction.set_chain_id(chain_id);

        // Force nonce to be set
        let nonce = u64::arbitrary(u)?;
        primitive_transaction.set_nonce(nonce);

        // Compute the signing hash
        let signing_hash = primitive_transaction.signature_hash();

        // Sign the transaction with a local wallet
        let signer = PrivateKeySigner::random();
        let signature = signer.sign_hash_sync(&signing_hash).map_err(|_| arbitrary::Error::IncorrectFormat)?;

        // Use TransactionSignedNoHash to compute the hash
        let y_parity = signature.v().y_parity();
        let hash = TransactionSignedNoHash {
            transaction: primitive_transaction.clone(),
            signature: reth_primitives::Signature { r: signature.r(), s: signature.s(), odd_y_parity: y_parity },
        }
        .hash();

        // Convert the signature to the RPC format
        let is_legacy = primitive_transaction.is_legacy();
        // See docs on `alloy::rpc::types::Signature` for `v` field.
        let v: u64 = if is_legacy { 35 + 2 * chain_id + u64::from(y_parity) } else { u64::from(y_parity) };
        let signature = alloy::rpc::types::Signature {
            r: signature.r(),
            s: signature.s(),
            v: U256::from(v),
            y_parity: if is_legacy { None } else { Some(signature.v().y_parity().into()) },
        };

        let transaction = Transaction {
            hash,
            from: signer.address(),
            block_hash: Some(B256::arbitrary(u)?),
            block_number: Some(u64::arbitrary(u)?),
            transaction_index: Some(u64::arbitrary(u)?),
            gas_price: Some(primitive_transaction.effective_gas_price(None)),
            gas: u128::from(primitive_transaction.gas_limit()),
            max_fee_per_gas: if is_legacy { None } else { Some(primitive_transaction.max_fee_per_gas()) },
            max_priority_fee_per_gas: primitive_transaction.max_priority_fee_per_gas(),
            signature: Some(signature),
            transaction_type: Some(primitive_transaction.tx_type() as u8),
            chain_id: primitive_transaction.chain_id(),
            nonce: primitive_transaction.nonce(),
            other: Default::default(),
            access_list: Some(reth_rpc_types::AccessList::arbitrary(u)?),
            ..Default::default()
        };

        Ok(Self { tx: transaction })
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct StoredPendingTransaction {
    /// Transaction object
    #[serde(deserialize_with = "crate::providers::eth_provider::database::types::serde::deserialize_intermediate")]
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

impl From<&StoredPendingTransaction> for Transaction {
    fn from(tx: &StoredPendingTransaction) -> Self {
        tx.tx.clone()
    }
}

impl Deref for StoredPendingTransaction {
    type Target = Transaction;

    fn deref(&self) -> &Self::Target {
        &self.tx
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
