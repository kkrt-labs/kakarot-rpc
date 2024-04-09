#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use reth_primitives::{constants::EMPTY_ROOT_HASH, SealedHeader, B64, U256, U64};
use reth_rpc_types::Header;
use serde::{Deserialize, Serialize};

/// A header as stored in the database
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, Eq)]
pub struct StoredHeader {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub header: Header,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> arbitrary::Arbitrary<'a> for StoredHeader {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let header = SealedHeader::arbitrary(u)?;

        Ok(StoredHeader {
            header: Header {
                hash: Some(header.hash()),
                parent_hash: header.parent_hash,
                uncles_hash: header.ommers_hash,
                miner: header.beneficiary,
                state_root: header.state_root,
                transactions_root: header.transactions_root,
                receipts_root: header.receipts_root,
                logs_bloom: header.logs_bloom,
                difficulty: header.difficulty,
                number: Some(U256::from(header.number)),
                gas_limit: U256::from(header.gas_limit),
                gas_used: U256::from(header.gas_used),
                timestamp: U256::from(header.timestamp),
                total_difficulty: Some(U256::arbitrary(u)?),
                extra_data: header.extra_data.clone(),
                mix_hash: Some(header.mix_hash),
                nonce: Some(B64::from(header.nonce)),
                base_fee_per_gas: header.base_fee_per_gas.map(U256::from),
                withdrawals_root: Some(EMPTY_ROOT_HASH),
                blob_gas_used: header.blob_gas_used.map(U64::from),
                excess_blob_gas: header.excess_blob_gas.map(U64::from),
                parent_beacon_block_root: header.parent_beacon_block_root,
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
