use crate::eth_provider::constant::STARKNET_MODULUS;
use crate::{eth_provider::error::EthereumDataFormatError, into_via_try_wrapper};
use reth_primitives::{BlockId as EthereumBlockId, BlockNumberOrTag, U256};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};

#[derive(Debug)]
pub struct EthBlockId(EthereumBlockId);

impl EthBlockId {
    pub const fn new(block_id: EthereumBlockId) -> Self {
        Self(block_id)
    }
}

impl TryFrom<EthBlockId> for StarknetBlockId {
    type Error = EthereumDataFormatError;
    fn try_from(eth_block_id: EthBlockId) -> Result<Self, Self::Error> {
        match eth_block_id.0 {
            // TODO: the conversion currently relies on a modulo operation to ensure compatibility with the StarkNet modulus.
            // A revisit of this line is suggested when hash values are calculated as specified in the Ethereum specification.error
            EthereumBlockId::Hash(hash) => Ok(Self::Hash(into_via_try_wrapper!(U256::from_be_slice(
                hash.block_hash.as_slice()
            )
            .wrapping_rem(STARKNET_MODULUS))?)),
            EthereumBlockId::Number(block_number_or_tag) => {
                let block_number_or_tag: EthBlockNumberOrTag = block_number_or_tag.into();
                Ok(block_number_or_tag.into())
            }
        }
    }
}

impl From<EthBlockId> for EthereumBlockId {
    fn from(eth_block_id: EthBlockId) -> Self {
        eth_block_id.0
    }
}

#[derive(Debug)]
pub struct EthBlockNumberOrTag(BlockNumberOrTag);

impl From<BlockNumberOrTag> for EthBlockNumberOrTag {
    fn from(block_number_or_tag: BlockNumberOrTag) -> Self {
        Self(block_number_or_tag)
    }
}

impl From<EthBlockNumberOrTag> for BlockNumberOrTag {
    fn from(eth_block_number_or_tag: EthBlockNumberOrTag) -> Self {
        eth_block_number_or_tag.0
    }
}

impl From<EthBlockNumberOrTag> for StarknetBlockId {
    fn from(block_number_or_tag: EthBlockNumberOrTag) -> Self {
        match block_number_or_tag.into() {
            BlockNumberOrTag::Latest | BlockNumberOrTag::Pending => {
                // We set to pending because in Starknet, a pending block is an unsealed block,
                // With a centralized sequencer, the latest block is the pending block being filled.
                Self::Tag(BlockTag::Pending)
            }
            BlockNumberOrTag::Safe | BlockNumberOrTag::Finalized => Self::Tag(BlockTag::Latest),
            BlockNumberOrTag::Earliest => Self::Number(0),
            BlockNumberOrTag::Number(number) => Self::Number(number),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::transaction::rpc_to_primitive_transaction;
    use std::str::FromStr;

    use reth_primitives::{Address, Block, Bloom, Bytes, TransactionSigned, B256, B64, U256};
    use reth_rpc_types::{other::OtherFields, Parity, Signature};

    fn base_rpc_header() -> reth_rpc_types::Header {
        reth_rpc_types::Header {
            parent_hash: B256::from_str(&format!("0x{:0>64}", "01")).unwrap(),
            uncles_hash: B256::from_str(&format!("0x{:0>64}", "02")).unwrap(),
            miner: Address::from_str(&format!("0x{:0>40}", "03")).unwrap(),
            state_root: B256::from_str(&format!("0x{:0>64}", "04")).unwrap(),
            transactions_root: B256::from_str(&format!("0x{:0>64}", "05")).unwrap(),
            receipts_root: B256::from_str(&format!("0x{:0>64}", "06")).unwrap(),
            withdrawals_root: None,
            logs_bloom: Bloom::ZERO,
            difficulty: U256::ZERO,
            base_fee_per_gas: Some(8),
            blob_gas_used: Some(9),
            excess_blob_gas: Some(10),
            extra_data: Bytes::default(),
            gas_limit: 11,
            gas_used: 12,
            hash: Some(B256::from_str(&format!("0x{:0>64}", "D")).unwrap()),
            mix_hash: Some(B256::from_str(&format!("0x{:0>64}", "E")).unwrap()),
            parent_beacon_block_root: Some(B256::from_str(&format!("0x{:0>64}", "F")).unwrap()),
            nonce: Some(B64::from_str(&format!("0x{:0>16}", "10")).unwrap()),
            number: Some(17),
            timestamp: 18,
            total_difficulty: None,
        }
    }

    fn base_rpc_transaction() -> reth_rpc_types::Transaction {
        let access_list = reth_rpc_types::AccessList::default();

        reth_rpc_types::Transaction {
            hash: B256::default(),
            nonce: 1,
            block_hash: None,
            block_number: None,
            transaction_index: Some(0),
            from: Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            to: Some(Address::from_str("0x0000000000000000000000000000000000000002").unwrap()),
            value: U256::from(100),
            gas_price: Some(20),
            gas: 21000,
            max_fee_per_gas: Some(30),
            max_priority_fee_per_gas: Some(10),
            max_fee_per_blob_gas: None,
            input: Bytes::from("1234"),
            signature: Some(Signature {
                r: U256::from(111),
                s: U256::from(222),
                v: U256::from(1),
                y_parity: Some(Parity(true)),
            }),
            chain_id: Some(1),
            blob_versioned_hashes: Some(vec![]),
            access_list: Some(access_list),
            transaction_type: Some(2),
            other: serde_json::from_str("{}").unwrap(),
        }
    }

    fn base_rpc_block() -> reth_rpc_types::Block {
        reth_rpc_types::Block {
            header: base_rpc_header(),
            uncles: Vec::default(),
            transactions: reth_rpc_types::BlockTransactions::Full(vec![
                base_rpc_transaction(),
                base_rpc_transaction(),
                base_rpc_transaction(),
            ]),
            size: None,
            withdrawals: None,
            other: OtherFields::default(),
        }
    }

    #[test]
    fn test_rpc_to_primitive_block() {
        let block = base_rpc_block();
        let primitive_block = Block::try_from(block).unwrap();
        assert_eq!(primitive_block.header.parent_hash, B256::from_str(&format!("0x{:0>64}", "01")).unwrap());
        assert_eq!(primitive_block.header.ommers_hash, B256::from_str(&format!("0x{:0>64}", "02")).unwrap());
        assert_eq!(primitive_block.header.beneficiary, Address::from_str(&format!("0x{:0>40}", "03")).unwrap());
        assert_eq!(primitive_block.header.state_root, B256::from_str(&format!("0x{:0>64}", "04")).unwrap());
        assert_eq!(primitive_block.header.transactions_root, B256::from_str(&format!("0x{:0>64}", "05")).unwrap());
        assert_eq!(primitive_block.header.receipts_root, B256::from_str(&format!("0x{:0>64}", "06")).unwrap());
        assert!(primitive_block.header.withdrawals_root.is_none());
        assert_eq!(primitive_block.header.logs_bloom, Bloom::ZERO);
        assert_eq!(primitive_block.header.difficulty, U256::ZERO);
        assert_eq!(primitive_block.header.base_fee_per_gas, Some(8));
        assert_eq!(primitive_block.header.blob_gas_used, Some(9u64));
        assert_eq!(primitive_block.header.excess_blob_gas, Some(10u64));
        assert_eq!(primitive_block.header.gas_limit, 11u64);
        assert_eq!(primitive_block.header.gas_used, 12u64);
        assert_eq!(primitive_block.header.mix_hash, B256::from_str(&format!("0x{:0>64}", "E")).unwrap());
        assert_eq!(
            primitive_block.header.nonce,
            u64::from_be_bytes(B64::from_str(&format!("0x{:0>16}", "10")).unwrap().0)
        );
        assert_eq!(primitive_block.header.number, 17u64);
        assert_eq!(primitive_block.header.timestamp, 18u64);
        assert_eq!(
            primitive_block.body,
            vec![
                TransactionSigned::from_transaction_and_signature(
                    rpc_to_primitive_transaction(base_rpc_transaction()).unwrap(),
                    reth_primitives::Signature {
                        r: base_rpc_transaction().signature.unwrap().r,
                        s: base_rpc_transaction().signature.unwrap().s,
                        odd_y_parity: base_rpc_transaction().signature.unwrap().y_parity.unwrap().0,
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    rpc_to_primitive_transaction(base_rpc_transaction()).unwrap(),
                    reth_primitives::Signature {
                        r: base_rpc_transaction().signature.unwrap().r,
                        s: base_rpc_transaction().signature.unwrap().s,
                        odd_y_parity: base_rpc_transaction().signature.unwrap().y_parity.unwrap().0,
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    rpc_to_primitive_transaction(base_rpc_transaction()).unwrap(),
                    reth_primitives::Signature {
                        r: base_rpc_transaction().signature.unwrap().r,
                        s: base_rpc_transaction().signature.unwrap().s,
                        odd_y_parity: base_rpc_transaction().signature.unwrap().y_parity.unwrap().0,
                    },
                )
            ]
        );
        assert!(primitive_block.withdrawals.is_none());
        assert_eq!(primitive_block.ommers, Vec::default());
    }
}
