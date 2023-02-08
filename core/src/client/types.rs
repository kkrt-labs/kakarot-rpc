use reth_primitives::rpc::transaction::eip2930::AccessListItem;
use reth_primitives::{Address, Bloom, Bytes, H256, H64, U256};
use reth_primitives::{H512, U64};
use serde::{ser::Error, Deserialize, Serialize, Serializer};
use std::{collections::BTreeMap, ops::Deref};

/// Transaction object
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Hash
    pub hash: H256,
    /// Nonce
    pub nonce: U256,
    /// Block hash
    pub block_hash: Option<H256>,
    /// Block number
    pub block_number: Option<U256>,
    /// Transaction Index
    pub transaction_index: Option<U256>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U256>,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<U256>,
    /// The miner's tip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<U256>,
    /// Gas
    pub gas: U256,
    /// Data
    pub input: Bytes,
    /// Creates contract
    pub creates: Option<Address>,
    /// Raw transaction data
    pub raw: Bytes,
    /// Public key of the signer.
    pub public_key: Option<H512>,
    /// The network id of the transaction, if any.
    pub chain_id: Option<U64>,
    /// The standardised V field of the signature (0 or 1).
    pub standard_v: U256,
    /// The standardised V field of the signature.
    pub v: U256,
    /// The R field of the signature.
    pub r: U256,
    /// The S field of the signature.
    pub s: U256,
    /// Pre-pay to warm storage access.
    #[cfg_attr(feature = "std", serde(skip_serializing_if = "Option::is_none"))]
    pub access_list: Option<Vec<AccessListItem>>,
    /// EIP-2718 type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U256>,
}

/// Block Transactions depending on the boolean attribute of `eth_getBlockBy*`
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockTransactions {
    /// Only hashes
    Hashes(Vec<H256>),
    /// Full transactions
    Full(Vec<Transaction>),
}

/// Block representation
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// Header of the block
    #[serde(flatten)]
    pub header: Header,
    /// Total difficulty
    pub total_difficulty: U256,
    /// Uncles' hashes
    pub uncles: Vec<H256>,
    /// Transactions
    pub transactions: BlockTransactions,
    /// Size in bytes
    pub size: Option<U256>,
    /// Base Fee for post-EIP1559 blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_fee_per_gas: Option<U256>,
}

/// Block header representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    /// Hash of the block
    pub hash: Option<H256>,
    /// Hash of the parent
    pub parent_hash: H256,
    /// Hash of the uncles
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: H256,
    /// Authors address
    pub author: Address,
    /// Alias of `author`
    pub miner: Address,
    /// State root hash
    pub state_root: H256,
    /// Transactions root hash
    pub transactions_root: H256,
    /// Transactions receipts root hash
    pub receipts_root: H256,
    /// Block number
    pub number: Option<U256>,
    /// Gas Used
    pub gas_used: U256,
    /// Gas Limit
    pub gas_limit: U256,
    /// Extra data
    pub extra_data: Bytes,
    /// Logs bloom
    pub logs_bloom: Bloom,
    /// Timestamp
    pub timestamp: U256,
    /// Difficulty
    pub difficulty: U256,
    /// Nonce
    pub nonce: Option<H64>,
    /// Size in bytes
    pub size: Option<U256>,

    pub mix_hash: H256,

    pub base_fee_per_gas: U256,
}

/// A Block representation that allows to include additional fields
pub type RichBlock = Rich<Block>;

/// Header representation with additional info.
pub type RichHeader = Rich<Header>;

/// Value representation with additional info
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Rich<T> {
    /// Standard value.
    pub inner: T,
    /// Additional fields that should be serialized into the `Block` object
    #[serde(flatten)]
    pub extra_info: BTreeMap<String, serde_json::Value>,
}

impl<T> Deref for Rich<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Serialize> Serialize for Rich<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.extra_info.is_empty() {
            return self.inner.serialize(serializer);
        }

        let inner = serde_json::to_value(&self.inner);
        let extras = serde_json::to_value(&self.extra_info);

        if let (Ok(serde_json::Value::Object(mut value)), Ok(serde_json::Value::Object(extras))) =
            (inner, extras)
        {
            value.extend(extras);
            value.serialize(serializer)
        } else {
            Err(S::Error::custom(
                "Unserializable structures: expected objects",
            ))
        }
    }
}
