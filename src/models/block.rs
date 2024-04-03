use reth_primitives::{BlockId as EthereumBlockId, BlockNumberOrTag, TransactionSigned, Withdrawals};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};

use super::transaction::rpc_transaction_to_primitive;
use crate::{eth_provider::error::EthereumDataFormatError, into_via_try_wrapper};

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
            EthereumBlockId::Hash(hash) => Ok(Self::Hash(into_via_try_wrapper!(hash.block_hash)?)),
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
        let block_number_or_tag = block_number_or_tag.into();
        match block_number_or_tag {
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

pub fn rpc_to_primitive_header(
    header: reth_rpc_types::Header,
) -> Result<reth_primitives::Header, EthereumDataFormatError> {
    Ok(reth_primitives::Header {
        base_fee_per_gas: header
            .base_fee_per_gas
            .map(|base_fee_per_gas| base_fee_per_gas.try_into().map_err(|_| EthereumDataFormatError::PrimitiveError))
            .transpose()?,
        beneficiary: header.miner,
        blob_gas_used: header.blob_gas_used.map(|blob_gas_used| blob_gas_used.to::<u64>()),
        difficulty: header.difficulty,
        excess_blob_gas: header.excess_blob_gas.map(|excess_blob_gas| excess_blob_gas.to::<u64>()),
        extra_data: header.extra_data,
        gas_limit: header.gas_limit.try_into().map_err(|_| EthereumDataFormatError::PrimitiveError)?,
        gas_used: header.gas_used.try_into().map_err(|_| EthereumDataFormatError::PrimitiveError)?,
        logs_bloom: header.logs_bloom,
        mix_hash: header.mix_hash.unwrap_or_default(),
        nonce: u64::from_be_bytes(header.nonce.unwrap_or_default().0),
        number: header
            .number
            .ok_or(EthereumDataFormatError::PrimitiveError)?
            .try_into()
            .map_err(|_| EthereumDataFormatError::PrimitiveError)?,
        ommers_hash: header.uncles_hash,
        parent_beacon_block_root: header.parent_beacon_block_root,
        parent_hash: header.parent_hash,
        receipts_root: header.receipts_root,
        state_root: header.state_root,
        timestamp: header.timestamp.try_into().map_err(|_| EthereumDataFormatError::PrimitiveError)?,
        transactions_root: header.transactions_root,
        withdrawals_root: header.withdrawals_root,
    })
}

pub fn rpc_to_primitive_block(block: reth_rpc_types::Block) -> Result<reth_primitives::Block, EthereumDataFormatError> {
    let body = {
        let transactions: Result<Vec<TransactionSigned>, EthereumDataFormatError> = match block.transactions {
            reth_rpc_types::BlockTransactions::Full(transactions) => transactions
                .into_iter()
                .map(|tx| {
                    let signature = tx.signature.ok_or(EthereumDataFormatError::PrimitiveError)?;
                    let tx_signed = TransactionSigned::from_transaction_and_signature(
                        rpc_transaction_to_primitive(tx)?,
                        reth_primitives::Signature {
                            r: signature.r,
                            s: signature.s,
                            odd_y_parity: signature.y_parity.unwrap_or(reth_rpc_types::Parity(false)).0,
                        },
                    );
                    Ok(tx_signed)
                })
                .collect(),
            reth_rpc_types::BlockTransactions::Hashes(_transaction_hashes) => {
                return Err(EthereumDataFormatError::PrimitiveError);
            }
            reth_rpc_types::BlockTransactions::Uncle => {
                return Err(EthereumDataFormatError::PrimitiveError);
            }
        };
        transactions?
    };
    // ⚠️ Kakarot does not support omners or withdrawals and returns default values for those fields ⚠️
    Ok(reth_primitives::Block {
        header: rpc_to_primitive_header(block.header)?,
        body,
        ommers: Default::default(),
        withdrawals: Some(Withdrawals::default()),
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reth_primitives::{Address, Bloom, Bytes, B256, B64, U128, U256, U64};
    use reth_rpc_types::{other::OtherFields, Parity, Signature};

    use super::*;

    fn base_rpc_header() -> reth_rpc_types::Header {
        reth_rpc_types::Header {
            parent_hash: B256::from_str(&format!("0x{:0>64}", "01")).unwrap(),
            uncles_hash: B256::from_str(&format!("0x{:0>64}", "02")).unwrap(),
            miner: Address::from_str(&format!("0x{:0>40}", "03")).unwrap(),
            state_root: B256::from_str(&format!("0x{:0>64}", "04")).unwrap(),
            transactions_root: B256::from_str(&format!("0x{:0>64}", "05")).unwrap(),
            receipts_root: B256::from_str(&format!("0x{:0>64}", "06")).unwrap(),
            withdrawals_root: Some(B256::from_str(&format!("0x{:0>64}", "07")).unwrap()),
            logs_bloom: Bloom::ZERO,
            difficulty: U256::ZERO,
            base_fee_per_gas: Some(U256::from(8)),
            blob_gas_used: Some(U64::from(9)),
            excess_blob_gas: Some(U64::from(10)),
            extra_data: Bytes::default(),
            gas_limit: U256::from(11),
            gas_used: U256::from(12),
            hash: Some(B256::from_str(&format!("0x{:0>64}", "D")).unwrap()),
            mix_hash: Some(B256::from_str(&format!("0x{:0>64}", "E")).unwrap()),
            parent_beacon_block_root: Some(B256::from_str(&format!("0x{:0>64}", "F")).unwrap()),
            nonce: Some(B64::from_str(&format!("0x{:0>16}", "10")).unwrap()),
            number: Some(U256::from(17)),
            timestamp: U256::from(18),
            total_difficulty: None,
        }
    }

    fn base_rpc_transaction() -> reth_rpc_types::Transaction {
        reth_rpc_types::Transaction {
            hash: B256::default(),
            nonce: U64::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: Some(U256::ZERO),
            from: Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            to: Some(Address::from_str("0x0000000000000000000000000000000000000002").unwrap()),
            value: U256::from(100),
            gas_price: Some(U128::from(20)),
            gas: U256::from(21000),
            max_fee_per_gas: Some(U128::from(30)),
            max_priority_fee_per_gas: Some(U128::from(10)),
            max_fee_per_blob_gas: None,
            input: Bytes::from("1234"),
            signature: Some(Signature {
                r: U256::from(111),
                s: U256::from(222),
                v: U256::from(1),
                y_parity: Some(Parity(true)),
            }),
            chain_id: Some(U64::from(1)),
            blob_versioned_hashes: vec![],
            access_list: None,
            transaction_type: Some(U64::from(2)),
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
            withdrawals: Some(Vec::default()),
            other: OtherFields::default(),
        }
    }

    #[test]
    fn test_rpc_to_primitive_block() {
        let block = base_rpc_block();
        let primitive_block = rpc_to_primitive_block(block).unwrap();
        assert_eq!(primitive_block.header.parent_hash, B256::from_str(&format!("0x{:0>64}", "01")).unwrap());
        assert_eq!(primitive_block.header.ommers_hash, B256::from_str(&format!("0x{:0>64}", "02")).unwrap());
        assert_eq!(primitive_block.header.beneficiary, Address::from_str(&format!("0x{:0>40}", "03")).unwrap());
        assert_eq!(primitive_block.header.state_root, B256::from_str(&format!("0x{:0>64}", "04")).unwrap());
        assert_eq!(primitive_block.header.transactions_root, B256::from_str(&format!("0x{:0>64}", "05")).unwrap());
        assert_eq!(primitive_block.header.receipts_root, B256::from_str(&format!("0x{:0>64}", "06")).unwrap());
        assert_eq!(
            primitive_block.header.withdrawals_root.unwrap(),
            B256::from_str(&format!("0x{:0>64}", "07")).unwrap()
        );
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
                    rpc_transaction_to_primitive(base_rpc_transaction()).unwrap(),
                    reth_primitives::Signature {
                        r: base_rpc_transaction().signature.unwrap().r,
                        s: base_rpc_transaction().signature.unwrap().s,
                        odd_y_parity: base_rpc_transaction().signature.unwrap().y_parity.unwrap().0,
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    rpc_transaction_to_primitive(base_rpc_transaction()).unwrap(),
                    reth_primitives::Signature {
                        r: base_rpc_transaction().signature.unwrap().r,
                        s: base_rpc_transaction().signature.unwrap().s,
                        odd_y_parity: base_rpc_transaction().signature.unwrap().y_parity.unwrap().0,
                    },
                ),
                TransactionSigned::from_transaction_and_signature(
                    rpc_transaction_to_primitive(base_rpc_transaction()).unwrap(),
                    reth_primitives::Signature {
                        r: base_rpc_transaction().signature.unwrap().r,
                        s: base_rpc_transaction().signature.unwrap().s,
                        odd_y_parity: base_rpc_transaction().signature.unwrap().y_parity.unwrap().0,
                    },
                )
            ]
        );
        assert_eq!(primitive_block.withdrawals, Some(Withdrawals::default()));
        assert_eq!(primitive_block.ommers, Vec::default());
    }
}
