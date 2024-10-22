use alloy_rpc_types::Header;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    alloy_primitives::{B256, B64, U256},
    arbitrary::Arbitrary,
    reth_primitives::constants::EMPTY_ROOT_HASH,
};

/// A header as stored in the database
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, Eq)]
pub struct StoredHeader {
    #[serde(deserialize_with = "crate::providers::eth_provider::database::types::serde::deserialize_intermediate")]
    pub header: Header,
}

impl From<StoredHeader> for Header {
    fn from(header: StoredHeader) -> Self {
        header.header
    }
}

impl From<&StoredHeader> for Header {
    fn from(header: &StoredHeader) -> Self {
        header.header.clone()
    }
}

impl Deref for StoredHeader {
    type Target = Header;

    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl Arbitrary<'_> for StoredHeader {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self {
            header: Header {
                hash: B256::arbitrary(u)?,
                total_difficulty: Some(U256::arbitrary(u).unwrap()),
                mix_hash: Some(B256::arbitrary(u).unwrap()),
                nonce: Some(B64::arbitrary(u).unwrap()),
                withdrawals_root: Some(EMPTY_ROOT_HASH),
                base_fee_per_gas: Some(u64::arbitrary(u).unwrap()),
                blob_gas_used: Some(u64::arbitrary(u).unwrap()),
                excess_blob_gas: Some(u64::arbitrary(u).unwrap()),
                gas_limit: u64::arbitrary(u).unwrap(),
                gas_used: u64::arbitrary(u).unwrap(),
                number: u64::arbitrary(u).unwrap(),
                ..Header::arbitrary(u)?
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
    fn test_stored_header_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _ = StoredHeader::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }
}
