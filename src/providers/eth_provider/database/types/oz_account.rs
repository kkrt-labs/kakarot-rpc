#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use arbitrary::Arbitrary;
use reth_primitives::B256;
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;

/// An account as stored in the database
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoredOzAccount {
    pub address: Felt,
    pub current_tx_hash: Option<B256>,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl Arbitrary<'_> for StoredOzAccount {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self { address: Felt::ONE, current_tx_hash: Some(B256::arbitrary(u)?) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn test_oz_account_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _ = StoredOzAccount::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }
}
