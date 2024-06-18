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
                base_fee_per_gas: Some(u128::from(u64::arbitrary(u).unwrap())),
                blob_gas_used: Some(u128::from(u64::arbitrary(u).unwrap())),
                excess_blob_gas: Some(u128::from(u64::arbitrary(u).unwrap())),
                gas_limit: u128::from(u64::arbitrary(u).unwrap()),
                gas_used: u128::from(u64::arbitrary(u).unwrap()),
                number: Some(u64::arbitrary(u).unwrap()),
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
