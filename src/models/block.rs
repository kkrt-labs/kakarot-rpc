use crate::{
    into_via_try_wrapper,
    providers::eth_provider::{constant::STARKNET_MODULUS, error::EthereumDataFormatError},
};
use alloy_primitives::U256;
use reth_primitives::{BlockId as EthereumBlockId, BlockNumberOrTag};
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
