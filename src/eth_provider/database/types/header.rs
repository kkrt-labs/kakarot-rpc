use reth_rpc_types::Header;
use serde::{Deserialize, Serialize};
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    arbitrary::Arbitrary,
    reth_primitives::{constants::EMPTY_ROOT_HASH, B256, B64, U256},
};

/// A header as stored in the database
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "arbitrary", feature = "testing"), derive(arbitrary::Arbitrary))]
pub struct StoredHeader {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub header: Header,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> StoredHeader {
    pub fn arbitrary_with_optional_fields(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            header: Header {
                hash: Some(B256::arbitrary(u)?),
                total_difficulty: Some(U256::arbitrary(u).unwrap()),
                mix_hash: Some(B256::arbitrary(u).unwrap()),
                nonce: Some(B64::arbitrary(u).unwrap()),
                withdrawals_root: Some(EMPTY_ROOT_HASH),
                base_fee_per_gas: Some(u64::arbitrary(u).unwrap() as u128),
                blob_gas_used: Some(u64::arbitrary(u).unwrap() as u128),
                excess_blob_gas: Some(u64::arbitrary(u).unwrap() as u128),
                gas_limit: u64::arbitrary(u).unwrap() as u128,
                gas_used: u64::arbitrary(u).unwrap() as u128,
                number: Some(u64::arbitrary(u).unwrap()),
                ..Self::arbitrary(u)?.header
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
