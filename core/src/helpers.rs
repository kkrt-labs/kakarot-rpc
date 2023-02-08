use eyre::Result;
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
    accounts::Call,
    core::types::FieldElement,
    providers::jsonrpc::models::{
        BlockId as StarknetBlockId, BlockTag, InvokeTransaction, MaybePendingBlockWithTxHashes,
        MaybePendingBlockWithTxs, Transaction as StarknetTransaction,
    },
};

use crate::client::{
    constants::{selectors::EXECUTE_AT_ADDRESS, CHAIN_ID, KAKAROT_MAIN_CONTRACT_ADDRESS},
    types::{Block, BlockTransactions, Header, Rich, RichBlock, Transaction as EtherTransaction},
    KakarotClientError,
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

#[derive(Debug, PartialEq)]
pub enum FeltOrFeltArray {
    Felt(FieldElement),
    FeltArray(Vec<FieldElement>),
}

pub fn ethers_block_id_to_starknet_block_id(
    block: EthBlockId,
) -> Result<StarknetBlockId, KakarotClientError> {
    match block {
        EthBlockId::Hash(hash) => {
            let address_hex = hex::encode(hash);
            let address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
                KakarotClientError::OtherError(anyhow::anyhow!(
                    "Failed to convert Starknet block hash to FieldElement: {}",
                    e
                ))
            })?;
            Ok(StarknetBlockId::Hash(address_felt))
        }
        EthBlockId::Number(number) => ethers_block_number_to_starknet_block_id(number),
    }
}

pub fn ethers_block_number_to_starknet_block_id(
    block: BlockNumber,
) -> Result<StarknetBlockId, KakarotClientError> {
    match block {
        BlockNumber::Latest => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
        BlockNumber::Finalized => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
        BlockNumber::Safe => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
        BlockNumber::Earliest => Ok(StarknetBlockId::Number(0)),
        BlockNumber::Pending => Ok(StarknetBlockId::Tag(BlockTag::Pending)),
        BlockNumber::Number(num) => Ok(StarknetBlockId::Number(num.as_u64())),
    }
}

pub fn starknet_block_to_eth_block(block: MaybePendingStarknetBlock) -> RichBlock {
    // Fixed fields in the Ethereum block as Starknet does not have these fields

    //InvokeTransactionReceipt -
    //TODO: Fetch real data
    let gas_limit = U256::from(1000000); // Hard Code
                                         //TODO: Fetch real data
    let gas_used = U256::from(500000); // Hard Code (Sum of actual_fee's)
                                       //TODO: Fetch real data
    let difficulty = U256::from(1000000); // Fixed
                                          //TODO: Fetch real data
    let nonce: Option<H64> = Some(H64::from_low_u64_be(0));
    //TODO: Fetch real data
    let size: Option<U256> = Some(U256::from(100));
    // Bloom is a byte array of length 256
    let logs_bloom = Bloom::default();
    let extra_data = Bytes::from(b"0x00");
    //TODO: Fetch real data
    let total_difficulty: U256 = U256::from(1000000);
    //TODO: Fetch real data
    let base_fee_per_gas = U256::from(32);
    //TODO: Fetch real data
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

pub fn decode_execute_at_address_return(
    call_result: Vec<FieldElement>,
) -> Result<Vec<FeltOrFeltArray>, KakarotClientError> {
    // Parse and decode Kakarot's call return data (temporary solution and not scalable - will
    // fail is Kakarot API changes)
    // Declare Vec of Result
    let mut segmented_result: Vec<FeltOrFeltArray> = Vec::new();
    let mut tmp_array_len: FieldElement = *call_result.get(0).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call",
        ))
    })?;
    let mut tmp_counter = 1_usize;
    segmented_result.push(FeltOrFeltArray::FeltArray(Vec::new()));
    // Parse first array: stack_accesses
    while tmp_array_len != FieldElement::ZERO {
        let element = call_result.get(tmp_counter).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode return arguments of Kakarot call: stack accesses array",
            ))
        })?;
        match segmented_result.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        tmp_counter += 1;
        tmp_array_len = tmp_array_len - FieldElement::from(1_u64);
    }
    // Parse stack_len
    let stack_len = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: stack_len"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*stack_len));
    tmp_counter += 1;
    // Parse second array: memory_accesses
    tmp_array_len = *(call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: memory_accesses_len",
        ))
    })?);
    segmented_result.push(FeltOrFeltArray::FeltArray(Vec::new()));
    tmp_counter += 1;
    while tmp_array_len != FieldElement::ZERO {
        let element = call_result.get(tmp_counter).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode return arguments of Kakarot call: memory accesses array",
            ))
        })?;
        match segmented_result.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        tmp_counter += 1;
        tmp_array_len = tmp_array_len - FieldElement::from(1_u64);
    }
    // Parse memory_len
    let memory_len = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: memory len"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*memory_len));
    tmp_counter += 1;
    // Parse EVM address
    let evm_address = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: evm address"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*evm_address));
    tmp_counter += 1;
    // Parse Starknet Address
    let starknet_address = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: starknet address"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*starknet_address));
    tmp_counter += 1;
    // Parse last array: return_data
    tmp_array_len = *(call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: return_data_len",
        ))
    })?);
    segmented_result.push(FeltOrFeltArray::FeltArray(Vec::new()));
    tmp_counter += 1;
    while tmp_array_len != FieldElement::ZERO {
        let element = call_result.get(tmp_counter).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode return arguments of Kakarot call: return data array",
            ))
        })?;
        match segmented_result.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        tmp_counter += 1;
        tmp_array_len = tmp_array_len - FieldElement::from(1_u64);
    }
    // Parse gas_used return value
    let gas_used = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: gas used"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*gas_used));

    Ok(segmented_result)
}

pub fn starknet_tx_into_eth_tx(
    tx: StarknetTransaction,
) -> Result<EtherTransaction, KakarotClientError> {
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

fn felt_option_to_u256(element: Option<&FieldElement>) -> Result<U256, KakarotClientError> {
    match element {
        Some(x) => {
            let inner = x.to_bytes_be();
            Ok(U256::from_be_bytes(inner))
        }
        None => Ok(U256::from(0)),
    }
}

fn felt_to_u256(element: FieldElement) -> U256 {
    let inner = element.to_bytes_be();
    U256::from_be_bytes(inner)
}

fn vec_felt_to_bytes(felt_vec: Vec<FieldElement>) -> Bytes {
    let felt_vec_in_u8: Vec<u8> = felt_vec.into_iter().flat_map(|x| x.to_bytes_be()).collect();
    Bytes::from(felt_vec_in_u8)
}

fn starknet_address_to_ethereum_address(x: FieldElement) -> Address {
    H160::from_slice(&x.to_bytes_be()[12..32])
}

pub fn bytes_to_felt_vec(bytes: Bytes) -> Vec<FieldElement> {
    bytes.to_vec().into_iter().map(FieldElement::from).collect()
}

/// Author: https://github.com/xJonathanLEI/starknet-rs/blob/447182a90839a3e4f096a01afe75ef474186d911/starknet-accounts/src/account/execution.rs#L166
/// Constructs the calldata for a raw Starknet invoke transaction call
/// ## Arguments
/// * `bytes` - The calldata to be passed to the contract - RLP encoded raw EVM transaction
///
///
/// ## Returns
/// * `Result<Vec<FieldElement>>` - The calldata for the raw Starknet invoke transaction call
pub fn raw_calldata(bytes: Bytes) -> Result<Vec<FieldElement>> {
    let kakarot_address_felt = FieldElement::from_hex_be(KAKAROT_MAIN_CONTRACT_ADDRESS)?;
    let calls: Vec<Call> = vec![Call {
        to: kakarot_address_felt,
        selector: EXECUTE_AT_ADDRESS,
        calldata: bytes_to_felt_vec(bytes),
    }];
    let mut concated_calldata: Vec<FieldElement> = vec![];
    let mut execute_calldata: Vec<FieldElement> = vec![calls.len().into()];
    for call in calls.iter() {
        execute_calldata.push(call.to); // to
        execute_calldata.push(call.selector); // selector
        execute_calldata.push(concated_calldata.len().into()); // data_offset
        execute_calldata.push(call.calldata.len().into()); // data_len

        for item in call.calldata.iter() {
            concated_calldata.push(*item);
        }
    }
    execute_calldata.push(concated_calldata.len().into()); // calldata_len
    for item in concated_calldata.into_iter() {
        execute_calldata.push(item); // calldata
    }

    Ok(execute_calldata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_felt_vec() {
        let bytes = Bytes::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let felt_vec = bytes_to_felt_vec(bytes);
        assert_eq!(felt_vec.len(), 10);
        assert_eq!(
            felt_vec,
            vec![
                FieldElement::from(1_u64),
                FieldElement::from(2_u64),
                FieldElement::from(3_u64),
                FieldElement::from(4_u64),
                FieldElement::from(5_u64),
                FieldElement::from(6_u64),
                FieldElement::from(7_u64),
                FieldElement::from(8_u64),
                FieldElement::from(9_u64),
                FieldElement::from(10_u64)
            ]
        );
    }

    #[test]
    fn test_decode_execute_at_address() {
        let call_result = vec![
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000009",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000006661abd",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000007",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0x000000000000000000000000abde1007e67126e0755af0ff0173f919738f8373",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0x062897a9e931ba1ae4721548bd963e3fe67126e0755af0ff0173f919738f8373",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000020",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "00000000000000000000000000000000000000000000000000000000000fffff",
            )
            .unwrap(),
        ];
        let result = decode_execute_at_address_return(call_result).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(
            result[0],
            FeltOrFeltArray::FeltArray(vec![FieldElement::from(1_u64), FieldElement::from(2_u64)])
        );
        assert_eq!(result[1], FeltOrFeltArray::Felt(FieldElement::from(9_u64)));
        assert_eq!(
            result[2],
            FeltOrFeltArray::FeltArray(vec![FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000006661abd",
            )
            .unwrap()])
        );
        assert_eq!(result[3], FeltOrFeltArray::Felt(FieldElement::from(7_u64)));
        assert_eq!(
            result[4],
            FeltOrFeltArray::Felt(
                FieldElement::from_hex_be(
                    "0x000000000000000000000000abde1007e67126e0755af0ff0173f919738f8373",
                )
                .unwrap(),
            )
        );
        assert_eq!(
            result[5],
            FeltOrFeltArray::Felt(
                FieldElement::from_hex_be(
                    "0x062897a9e931ba1ae4721548bd963e3fe67126e0755af0ff0173f919738f8373",
                )
                .unwrap(),
            )
        );
        let mut return_data_vec = Vec::new();
        for _ in 0..31 {
            return_data_vec.push(FieldElement::from(0_u64));
        }
        return_data_vec.push(FieldElement::from(2_u64));
        assert_eq!(result[6], FeltOrFeltArray::FeltArray(return_data_vec));
        if let FeltOrFeltArray::FeltArray(felt_array) = &result[6] {
            assert_eq!(felt_array.len(), 32);
        } else {
            panic!("Expected FeltArray of length 32");
        }
        assert_eq!(
            result[7],
            FeltOrFeltArray::Felt(FieldElement::from(0x00000fffff_u64))
        )
    }
}
