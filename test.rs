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
    let mix_hash = PrimitiveH256::from_low_u64_be(0);

    match block {
        MaybePendingStarknetBlock::BlockWithTxHashes(maybe_pending_block) => {
            match maybe_pending_block {
                MaybePendingBlockWithTxHashes::PendingBlock(pending_block_with_tx_hashes) => {
                    let parent_hash =
                        PrimitiveH256::from_slice(&pending_block_with_tx_hashes.parent_hash.to_bytes_be());
                    let sequencer = H160::from_slice(
                        &pending_block_with_tx_hashes.sequencer_address.to_bytes_be()[12..32],
                    );
                    let timestamp =
                        U256::from_be_bytes(pending_block_with_tx_hashes.timestamp.to_be_bytes());
                    let transactions = BlockTransactions::Hashes(
                        pending_block_with_tx_hashes
                            .transactions
                            .into_iter()
                            .map(|tx| PrimitiveH256::from_slice(&tx.to_bytes_be()))
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
                        state_root: PrimitiveH256::zero(),
                        // PendingblockWithTxHashes doesn't have a transactions root
                        transactions_root: PrimitiveH256::zero(),
                        // PendingblockWithTxHashes doesn't have a receipts root
                        receipts_root: PrimitiveH256::zero(),
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
                    let hash = PrimitiveH256::from_slice(&block_with_tx_hashes.block_hash.to_bytes_be());
                    let parent_hash =
                        PrimitiveH256::from_slice(&block_with_tx_hashes.parent_hash.to_bytes_be());
                    let sequencer = H160::from_slice(
                        &block_with_tx_hashes.sequencer_address.to_bytes_be()[12..32],
                    );
                    let state_root = PrimitiveH256::from_slice(&block_with_tx_hashes.new_root.to_bytes_be());
                    let number = U256::from(block_with_tx_hashes.block_number);
                    let timestamp = U256::from(block_with_tx_hashes.timestamp);
                    let transactions = BlockTransactions::Hashes(
                        block_with_tx_hashes
                            .transactions
                            .into_iter()
                            .map(|tx| PrimitiveH256::from_slice(&tx.to_bytes_be()))
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
                        transactions_root: PrimitiveH256::zero(),
                        // BlockWithTxHashes doesn't have a receipts root
                        receipts_root: PrimitiveH256::zero(),
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
                    PrimitiveH256::from_slice(&pending_block_with_txs.parent_hash.to_bytes_be());
                let sequencer = H160::from_slice(
                    &pending_block_with_txs.sequencer_address.to_bytes_be()[12..32],
                );
                let timestamp = U256::from_be_bytes(pending_block_with_txs.timestamp.to_be_bytes());
                let transactions = BlockTransactions::Full(
                    pending_block_with_txs
                        .transactions
                        .into_iter()
                        .map(|t| starknet_tx_into_eth_tx(t, None, None))
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
                    state_root: PrimitiveH256::zero(),
                    // PendingBlockWithTxs doesn't have a transactions root
                    transactions_root: PrimitiveH256::zero(),
                    // PendingBlockWithTxs doesn't have a receipts root
                    receipts_root: PrimitiveH256::zero(),
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
                let hash = PrimitiveH256::from_slice(&block_with_txs.block_hash.to_bytes_be());
                let parent_hash = PrimitiveH256::from_slice(&block_with_txs.parent_hash.to_bytes_be());
                let sequencer =
                    H160::from_slice(&block_with_txs.sequencer_address.to_bytes_be()[12..32]);
                let state_root = PrimitiveH256::from_slice(&block_with_txs.new_root.to_bytes_be());
                let transactions_root = PrimitiveH256::from_slice(
                    &"0xac91334ba861cb94cba2b1fd63df7e87c15ca73666201abd10b5462255a5c642"
                        .as_bytes()[1..33],
                );
                let receipts_root = PrimitiveH256::from_slice(
                    &"0xf2c8755adf35e78ffa84999e48aba628e775bb7be3c70209738d736b67a9b549"
                        .as_bytes()[1..33],
                );

                let number = U256::from(block_with_txs.block_number);
                let timestamp = U256::from(block_with_txs.timestamp);

                let blockhash_opt =
                    Some(PrimitiveH256::from_slice(&(block_with_txs.block_hash).to_bytes_be()));
                let blocknum_opt = Some(U256::from(block_with_txs.block_number));
                let transactions = BlockTransactions::Full(
                    block_with_txs
                        .transactions
                        .into_iter()
                        .map(|t| starknet_tx_into_eth_tx(t, blockhash_opt, blocknum_opt))
                        .filter_map(Result::ok)
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