use eyre::{ErrReport, Error, Result};
use reth_primitives::{
    rpc::{BlockId as EthBlockId, BlockNumber},
    Bloom, Bytes, H160, H256, H64, U256,
};
use std::collections::BTreeMap;

use reth_primitives::Address;
// use reth_rpc_types::{
//     Block, BlockTransactions, Header, Rich, Transaction as EtherTransaction,
// };
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::models::{
        BlockId as StarknetBlockId, BlockTag, InvokeTransaction, MaybePendingBlockWithTxHashes,
        MaybePendingBlockWithTxs, Transaction as StarknetTransaction,
    },
};

use crate::lightclient::{
    constants::CHAIN_ID,
    types::{Block, BlockTransactions, Header, Rich, RichBlock, Transaction as EtherTransaction},
    LightClientError,
};

extern crate hex;

#[derive(Debug)]
struct InvalidFieldElementError;

impl std::error::Error for InvalidFieldElementError {}

impl std::fmt::Display for InvalidFieldElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid FieldElement")
    }
}

pub enum MaybePendingStarknetBlock {
    BlockWithTxHashes(MaybePendingBlockWithTxHashes),
    BlockWithTxs(MaybePendingBlockWithTxs),
}

pub fn ethers_block_id_to_starknet_block_id(
    block: EthBlockId,
) -> Result<StarknetBlockId, LightClientError> {
    match block {
        EthBlockId::Hash(hash) => {
            let address_hex = hex::encode(hash);
            let address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
                LightClientError::OtherError(anyhow::anyhow!(
                    "Failed to convert Starknet block hash to FieldElement: {}",
                    e
                ))
            })?;
            Ok(StarknetBlockId::Hash(address_felt))
        }
        EthBlockId::Number(number) => match number {
            BlockNumber::Latest => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
            BlockNumber::Finalized => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
            BlockNumber::Safe => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
            BlockNumber::Earliest => Ok(StarknetBlockId::Number(0)),
            BlockNumber::Pending => Ok(StarknetBlockId::Tag(BlockTag::Pending)),
            BlockNumber::Number(num) => Ok(StarknetBlockId::Number(num.as_u64())),
        },
    }
}

pub fn starknet_block_to_eth_block(block: MaybePendingStarknetBlock) -> RichBlock {
    // Fixed fields in the Ethereum block as Starknet does not have these fields

    //InvokeTransactionReceipt -
    let gas_limit = U256::from(1000000); // Hard Code
    let gas_used = U256::from(500000); // Hard Code (Sum of actual_fee's)
    let difficulty = U256::from(1000000); // Fixed
    let nonce: Option<H64> = Some(H64::from_low_u64_be(0));
    let size: Option<U256> = Some(U256::from(100));
    // Bloom is a byte array of length 256
    let logs_bloom = Bloom::default();
    let extra_data = Bytes::from(b"0x00");
    let total_difficulty: U256 = U256::from(1000000);
    let base_fee_per_gas = U256::from(32);
    let mix_hash = H256::from_low_u64_be(0);

    match block {
        MaybePendingStarknetBlock::BlockWithTxHashes(maybe_pending_block) => {
            match maybe_pending_block {
                MaybePendingBlockWithTxHashes::PendingBlock(pending_block_with_tx_hashes) => {
                    let parent_hash =
                        H256::from_slice(&pending_block_with_tx_hashes.parent_hash.to_bytes_be());
                    let sequencer = H160::from_slice(
                        &pending_block_with_tx_hashes.sequencer_address.to_bytes_be()[12..32],
                    );
                    let timestamp =
                        U256::from_be_bytes(pending_block_with_tx_hashes.timestamp.to_be_bytes());
                    let transactions = BlockTransactions::Hashes(
                        pending_block_with_tx_hashes
                            .transactions
                            .into_iter()
                            .map(|tx| H256::from_slice(&tx.to_bytes_be()))
                            .collect(),
                    );
                    let header = Header {
                        // PendingblockWithTxHashes doesn't have a block hash
                        hash: None,
                        parent_hash,
                        uncles_hash: parent_hash,
                        author: sequencer,
                        miner: sequencer,
                        // PendingblockWithTxHashes doesn't have a state root
                        state_root: H256::zero(),
                        // PendingblockWithTxHashes doesn't have a transactions root
                        transactions_root: H256::zero(),
                        // PendingblockWithTxHashes doesn't have a receipts root
                        receipts_root: H256::zero(),
                        // PendingblockWithTxHashes doesn't have a block number
                        number: None,
                        gas_used,
                        gas_limit,
                        extra_data,
                        logs_bloom,
                        timestamp,
                        difficulty,
                        nonce,
                        size,
                        base_fee_per_gas,
                        mix_hash,
                    };
                    let block = Block {
                        header,
                        total_difficulty,
                        uncles: vec![],
                        transactions,
                        base_fee_per_gas: None,
                        size,
                    };
                    Rich::<Block> {
                        inner: block,
                        extra_info: BTreeMap::default(),
                    }
                }
                MaybePendingBlockWithTxHashes::Block(block_with_tx_hashes) => {
                    let hash = H256::from_slice(&block_with_tx_hashes.block_hash.to_bytes_be());
                    let parent_hash =
                        H256::from_slice(&block_with_tx_hashes.parent_hash.to_bytes_be());
                    let sequencer = H160::from_slice(
                        &block_with_tx_hashes.sequencer_address.to_bytes_be()[12..32],
                    );
                    let state_root = H256::from_slice(&block_with_tx_hashes.new_root.to_bytes_be());
                    let number = U256::from(block_with_tx_hashes.block_number);
                    let timestamp = U256::from(block_with_tx_hashes.timestamp);
                    let transactions = BlockTransactions::Hashes(
                        block_with_tx_hashes
                            .transactions
                            .into_iter()
                            .map(|tx| H256::from_slice(&tx.to_bytes_be()))
                            .collect(),
                    );
                    let header = Header {
                        hash: Some(hash),
                        parent_hash,
                        uncles_hash: parent_hash,
                        author: sequencer,
                        miner: sequencer,
                        state_root,
                        // BlockWithTxHashes doesn't have a transactions root
                        transactions_root: H256::zero(),
                        // BlockWithTxHashes doesn't have a receipts root
                        receipts_root: H256::zero(),
                        number: Some(number),
                        gas_used,
                        gas_limit,
                        extra_data,
                        logs_bloom,
                        timestamp,
                        difficulty,
                        nonce,
                        size,
                        base_fee_per_gas,
                        mix_hash,
                    };
                    let block = Block {
                        header,
                        total_difficulty,
                        uncles: vec![],
                        transactions,
                        base_fee_per_gas: None,
                        size,
                    };
                    Rich::<Block> {
                        inner: block,
                        extra_info: BTreeMap::default(),
                    }
                }
            }
        }
        MaybePendingStarknetBlock::BlockWithTxs(maybe_pending_block) => match maybe_pending_block {
            MaybePendingBlockWithTxs::PendingBlock(pending_block_with_txs) => {
                let parent_hash =
                    H256::from_slice(&pending_block_with_txs.parent_hash.to_bytes_be());
                let sequencer = H160::from_slice(
                    &pending_block_with_txs.sequencer_address.to_bytes_be()[12..32],
                );
                let timestamp = U256::from_be_bytes(pending_block_with_txs.timestamp.to_be_bytes());
                let transactions = BlockTransactions::Full(
                    pending_block_with_txs
                        .transactions
                        .into_iter()
                        .map(starknet_tx_into_eth_tx)
                        .filter_map(Result::ok)
                        .collect(),
                );
                let header = Header {
                    // PendingBlockWithTxs doesn't have a block hash
                    hash: None,
                    parent_hash,
                    uncles_hash: parent_hash,
                    author: sequencer,
                    miner: sequencer,
                    // PendingBlockWithTxs doesn't have a state root
                    state_root: H256::zero(),
                    // PendingBlockWithTxs doesn't have a transactions root
                    transactions_root: H256::zero(),
                    // PendingBlockWithTxs doesn't have a receipts root
                    receipts_root: H256::zero(),
                    // PendingBlockWithTxs doesn't have a block number
                    number: None,
                    gas_used,
                    gas_limit,
                    extra_data,
                    logs_bloom,
                    timestamp,
                    difficulty,
                    nonce,
                    size,
                    base_fee_per_gas,
                    mix_hash,
                };
                let block = Block {
                    header,
                    total_difficulty,
                    uncles: vec![],
                    transactions,
                    base_fee_per_gas: None,
                    size,
                };
                Rich::<Block> {
                    inner: block,
                    extra_info: BTreeMap::default(),
                }
            }
            MaybePendingBlockWithTxs::Block(block_with_txs) => {
                println!("1. Calling Block With Txs");
                let hash = H256::from_slice(&block_with_txs.block_hash.to_bytes_be());
                let parent_hash = H256::from_slice(&block_with_txs.parent_hash.to_bytes_be());
                let sequencer =
                    H160::from_slice(&block_with_txs.sequencer_address.to_bytes_be()[12..32]);
                let state_root = H256::from_slice(&block_with_txs.new_root.to_bytes_be());
                let transactions_root = H256::from_slice(
                    &"0xac91334ba861cb94cba2b1fd63df7e87c15ca73666201abd10b5462255a5c642"
                        .as_bytes()[1..33],
                );
                let receipts_root = H256::from_slice(
                    &"0xf2c8755adf35e78ffa84999e48aba628e775bb7be3c70209738d736b67a9b549"
                        .as_bytes()[1..33],
                );

                let number = U256::from(block_with_txs.block_number);
                let timestamp = U256::from(block_with_txs.timestamp);
                println!("2. Getting transactions");

                let transactions = BlockTransactions::Full(
                    block_with_txs
                        .transactions
                        .into_iter()
                        .map(starknet_tx_into_eth_tx)
                        .filter_map(Result::ok)
                        .collect(),
                );
                println!("3. After Getting transactions");

                let header = Header {
                    hash: Some(hash),
                    parent_hash,
                    uncles_hash: parent_hash,
                    author: sequencer,
                    miner: sequencer,
                    state_root,
                    // BlockWithTxHashes doesn't have a transactions root
                    transactions_root,
                    // BlockWithTxHashes doesn't have a receipts root
                    receipts_root,
                    number: Some(number),
                    gas_used,
                    gas_limit,
                    extra_data,
                    logs_bloom,
                    timestamp,
                    difficulty,
                    nonce,
                    size,
                    base_fee_per_gas,
                    mix_hash,
                };
                let block = Block {
                    header,
                    total_difficulty,
                    uncles: vec![],
                    transactions,
                    base_fee_per_gas: None,
                    size,
                };
                Rich::<Block> {
                    inner: block,
                    extra_info: BTreeMap::default(),
                }
            }
        },
    }
}

fn starknet_tx_into_eth_tx(tx: StarknetTransaction) -> Result<EtherTransaction, Error> {
    let mut ether_tx = EtherTransaction::default();
    println!("2.1 Inside Getting transactions");

    match tx {
        StarknetTransaction::Invoke(invoke_tx) => {
            match invoke_tx {
                InvokeTransaction::V0(v0) => {
                    println!("2.X Inside InvokeV0");

                    // Extract relevant fields from InvokeTransactionV0 and convert them to the corresponding fields in EtherTransaction
                    ether_tx.hash = H256::from_slice(&v0.transaction_hash.to_bytes_be());
                    ether_tx.nonce = felt_to_u256(v0.nonce);
                    ether_tx.from = starknet_address_to_ethereum_address(v0.contract_address);
                    // Define gas_price data
                    ether_tx.gas_price = None;
                    // Extracting the signature
                    ether_tx.r = felt_option_to_u256(v0.signature.get(0))?;
                    ether_tx.s = felt_option_to_u256(v0.signature.get(1))?;
                    ether_tx.v = felt_option_to_u256(v0.signature.get(2))?;
                    // Extracting the data (transform from calldata)
                    ether_tx.input = vec_felt_to_bytes(v0.calldata);
                    //TODO:  Fetch transaction To
                    ether_tx.to = None;
                    //TODO:  Fetch value
                    ether_tx.value = U256::from(100);
                    //TODO: Fetch Gas
                    ether_tx.gas = U256::from(100);
                    // Extracting the chain_id
                    ether_tx.chain_id = Some(CHAIN_ID.into());
                    // Extracting the standard_v
                    ether_tx.standard_v = U256::from(0);
                    // Extracting the creates
                    ether_tx.creates = None;
                    // How to fetch the public_key?
                    ether_tx.public_key = None;
                    // ...
                }

                InvokeTransaction::V1(v1) => {
                    // Extract relevant fields from InvokeTransactionV0 and convert them to the corresponding fields in EtherTransaction
                    println!("2.X Inside InvokeV1");

                    ether_tx.hash = H256::from_slice(&v1.transaction_hash.to_bytes_be());
                    ether_tx.nonce = felt_to_u256(v1.nonce);
                    ether_tx.from = starknet_address_to_ethereum_address(v1.sender_address);
                    // Define gas_price data
                    ether_tx.gas_price = None;
                    // Extracting the signature
                    ether_tx.r = felt_option_to_u256(v1.signature.get(0))?;
                    ether_tx.s = felt_option_to_u256(v1.signature.get(1))?;
                    ether_tx.v = felt_option_to_u256(v1.signature.get(2))?;
                    // Extracting the data
                    ether_tx.input = vec_felt_to_bytes(v1.calldata);
                    // Extracting the to address
                    // TODO: Get Data from Calldata
                    ether_tx.to = None;
                    // Extracting the value
                    ether_tx.value = U256::from(100);
                    // TODO:: Get Gas from Estimate
                    ether_tx.gas = U256::from(100);
                    // Extracting the chain_id
                    ether_tx.chain_id = Some(CHAIN_ID.into());
                    // Extracting the standard_v
                    ether_tx.standard_v = U256::from(0);
                    // Extracting the creates
                    ether_tx.creates = None;
                    // Extracting the public_key
                    ether_tx.public_key = None;
                    // Extracting the access_list
                    ether_tx.access_list = None;
                    // Extracting the transaction_type
                    ether_tx.transaction_type = None;
                }
            }
        }
        // Repeat the process for each variant of StarknetTransaction
        StarknetTransaction::L1Handler(l1_handler_tx) => {
            // Extract relevant fields from InvokeTransactionV0 and convert them to the corresponding fields in EtherTransaction
            ether_tx.hash = H256::from_slice(&l1_handler_tx.transaction_hash.to_bytes_be());
            ether_tx.nonce = U256::from(l1_handler_tx.nonce);
            ether_tx.from = starknet_address_to_ethereum_address(l1_handler_tx.contract_address);
            // Define gas_price data
            ether_tx.gas_price = None;
            // Extracting the data
            ether_tx.input = vec_felt_to_bytes(l1_handler_tx.calldata);
            // Extracting the to address
            ether_tx.to = None;
            // Extracting the value
            ether_tx.value = U256::from(0);
            // TODO: Get from estimate gas
            ether_tx.gas = U256::from(0);
            // Extracting the chain_id
            ether_tx.chain_id = Some(CHAIN_ID.into());
            // Extracting the creates
            ether_tx.creates = None;
            // Extracting the public_key
            ether_tx.public_key = None;
        }
        StarknetTransaction::Declare(declare_tx) => {
            // Extract relevant fields from InvokeTransactionV0 and convert them to the corresponding fields in EtherTransaction
            ether_tx.hash = H256::from_slice(&declare_tx.transaction_hash.to_bytes_be());
            ether_tx.nonce = felt_to_u256(declare_tx.nonce);
            ether_tx.from = starknet_address_to_ethereum_address(declare_tx.sender_address);
            // Define gas_price data
            ether_tx.gas_price = None;
            // Extracting the signature
            ether_tx.r = felt_option_to_u256(declare_tx.signature.get(0))?;
            ether_tx.s = felt_option_to_u256(declare_tx.signature.get(1))?;
            ether_tx.v = felt_option_to_u256(declare_tx.signature.get(2))?;
            // Extracting the to address
            ether_tx.to = None;
            // Extracting the value
            ether_tx.value = U256::from(0);
            // Extracting the gas
            ether_tx.gas = U256::from(0);
            // Extracting the chain_id
            ether_tx.chain_id = Some(CHAIN_ID.into());
            // Extracting the standard_v
            ether_tx.standard_v = U256::from(0);
            // Extracting the public_key
            ether_tx.public_key = None;
        }
        StarknetTransaction::Deploy(deploy_tx) => {
            // Extract relevant fields from InvokeTransactionV0 and convert them to the corresponding fields in EtherTransaction
            ether_tx.hash = H256::from_slice(&deploy_tx.transaction_hash.to_bytes_be());
            // Define gas_price data
            ether_tx.gas_price = None;

            ether_tx.creates = None;
            // Extracting the public_key
            ether_tx.public_key = None;
        }
        StarknetTransaction::DeployAccount(deploy_account_tx) => {
            ether_tx.hash = H256::from_slice(&deploy_account_tx.transaction_hash.to_bytes_be());
            ether_tx.nonce = felt_to_u256(deploy_account_tx.nonce);
            // TODO: Get from estimate gas
            ether_tx.gas_price = None;
            // Extracting the signature
            ether_tx.r = felt_option_to_u256(deploy_account_tx.signature.get(0))?;
            ether_tx.s = felt_option_to_u256(deploy_account_tx.signature.get(1))?;
            ether_tx.v = felt_option_to_u256(deploy_account_tx.signature.get(2))?;
            // Extracting the to address
            ether_tx.to = None;
            // Extracting the gas
            ether_tx.gas = U256::from(0);
            // Extracting the chain_id
            ether_tx.chain_id = Some(CHAIN_ID.into());
            // Extracting the standard_v
            ether_tx.standard_v = U256::from(0);
            // Extracting the public_key
            ether_tx.public_key = None;
        }
    }
    println!("2.2 Before Returning Inside Getting transactions");

    Ok(ether_tx)
}

fn felt_option_to_u256(element: Option<&FieldElement>) -> Result<U256, Error> {
    match element {
        Some(x) => {
            let inner = x.to_bytes_be();
            Ok(U256::from_be_bytes(inner))
        }
        None => Err(ErrReport::new(InvalidFieldElementError)),
    }
}

fn felt_to_u256(element: FieldElement) -> U256 {
    let inner = element.to_bytes_be();
    U256::from_be_bytes(inner)
}

fn vec_felt_to_bytes(contract_bytecode: Vec<FieldElement>) -> Bytes {
    let contract_bytecode_in_u8: Vec<u8> = contract_bytecode
        .into_iter()
        .flat_map(|x| x.to_bytes_be())
        .collect();
    Bytes::from(contract_bytecode_in_u8)
}

fn starknet_address_to_ethereum_address(x: FieldElement) -> Address {
    H160::from_slice(&x.to_bytes_be()[12..32])
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use reth_primitives::rpc::{BlockId as EthBlockId, BlockNumber};
//     use starknet::providers::jsonrpc::models::BlockId as StarknetBlockId;
//     extern crate hex;

//     #[test]
//     fn test_ethers_block_id_to_starknet_block_id() {
//         let block_id = EthBlockId::Number(BlockNumber::Number(1.into()));
//         let starknet_block_id: StarknetBlockId =
//             ethers_block_id_to_starknet_block_id(block_id).unwrap();
//         assert_eq!(starknet_block_id, StarknetBlockId::Number(1));
//     }
// }
