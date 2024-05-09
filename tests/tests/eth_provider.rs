#![cfg(feature = "testing")]
use std::str::FromStr;

use kakarot_rpc::eth_provider::constant::{HASH_HEX_STRING_LEN, STARKNET_MODULUS, TRANSACTION_MAX_RETRIES};
use kakarot_rpc::eth_provider::database::types::transaction::{StoredPendingTransaction, StoredTransaction};
use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::eth_provider::utils::into_filter;
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::test_utils::eoa::Eoa as _;
use kakarot_rpc::test_utils::evm_contract::EvmContract;
use kakarot_rpc::test_utils::fixtures::{contract_empty, counter, katana, setup};
use kakarot_rpc::test_utils::mongo::{BLOCK_HASH, BLOCK_NUMBER};
use kakarot_rpc::test_utils::{evm_contract::KakarotEvmContract, katana::Katana};
use reth_primitives::serde_helper::{JsonStorageKey, U64};
use reth_primitives::transaction::Signature;
use reth_primitives::{sign_message, Transaction, TxKind, TxEip1559};
use reth_primitives::{Address, BlockNumberOrTag, Bytes, TransactionSigned, B256, U256, U64};
use reth_rpc_types::request::TransactionInput;
use reth_rpc_types::{Filter, FilterChanges, RpcBlockHash, TransactionRequest};
use rstest::*;
use starknet::core::types::BlockTag;
use starknet_crypto::FieldElement;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_number(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let block_number = eth_provider.block_number().await.unwrap();

    // Then
    // Catch the most recent block number of the mocked Mongo Database
    let expected = U64::from(katana.most_recent_transaction().unwrap().block_number.unwrap());
    assert_eq!(block_number, expected);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_chain_id(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let chain_id = eth_provider.chain_id().await.unwrap().unwrap_or_default();

    // Then
    // ASCII code for "test" is 0x74657374
    // Since kaka_test > u32::MAX, we should return the last 4 bytes of the chain_id.
    assert_eq!(chain_id, U64::from(0x74657374u64));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_by_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let block_hash = katana.most_recent_transaction().unwrap().block_hash.unwrap();

    // When
    let block = eth_provider.block_by_hash(block_hash, false).await.unwrap().unwrap();

    // Then
    assert_eq!(block.inner.header, katana.header_by_hash(block_hash).unwrap());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_by_number(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let block_number = katana.most_recent_transaction().unwrap().block_number.unwrap();

    // When: Retrieving block by specific block number
    let block = eth_provider.block_by_number(BlockNumberOrTag::Number(block_number), false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the expected block number
    assert_eq!(block.header.number, Some(block_number));

    // When: Retrieving earliest block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Earliest, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has block number zero
    assert_eq!(block.header.number, Some(0));

    // When: Retrieving latest block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Latest, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the same block number as the most recent transaction
    assert_eq!(block.header.number, Some(block_number));

    // When: Retrieving finalized block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Finalized, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the same block number as the most recent transaction
    assert_eq!(block.header.number, Some(block_number));

    // When: Retrieving safe block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Safe, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the same block number as the most recent transaction
    assert_eq!(block.header.number, Some(block_number));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_transaction_count_by_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let count = eth_provider.block_transaction_count_by_hash(*BLOCK_HASH).await.unwrap().unwrap();

    // Then
    assert_eq!(count, U256::from(3));

    // When
    let count = eth_provider
        .block_transaction_count_by_hash(katana.most_recent_transaction().unwrap().block_hash.unwrap())
        .await
        .unwrap()
        .unwrap();

    // Then
    assert_eq!(count, U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_transaction_count_by_number(#[future] katana: Katana, _setup: ()) {
    // Given: Ethereum provider instance
    let eth_provider = katana.eth_provider();

    // When: Retrieving transaction count for a specific block number
    let count =
        eth_provider.block_transaction_count_by_number(BlockNumberOrTag::Number(BLOCK_NUMBER)).await.unwrap().unwrap();

    // Then: Ensure the retrieved transaction count matches the expected value
    assert_eq!(count, U256::from(3));

    // When: Retrieving transaction count for the block of the most recent transaction
    let block_number = katana.most_recent_transaction().unwrap().block_number.unwrap();
    let count =
        eth_provider.block_transaction_count_by_number(BlockNumberOrTag::Number(block_number)).await.unwrap().unwrap();

    // Then: Ensure the retrieved transaction count matches the expected value
    assert_eq!(count, U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_balance(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();

    // When
    let eoa_balance = eth_provider.balance(eoa.evm_address().unwrap(), None).await.unwrap();

    // Then
    assert!(eoa_balance > U256::ZERO);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_storage_at(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Then
    let count = eth_provider.storage_at(counter_address, JsonStorageKey::from(U256::from(0)), None).await.unwrap();
    assert_eq!(B256::left_padding_from(&[0x1]), count);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce_eoa(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let nonce = eth_provider.transaction_count(Address::ZERO, None).await.unwrap();

    // Then
    // Zero address shouldn't throw 'ContractNotFound', but return zero
    assert_eq!(U256::from(0), nonce);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce_contract_account(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    let nonce_initial = eth_provider.transaction_count(counter_address, None).await.unwrap();

    // Then
    assert_eq!(nonce_initial, U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana: Katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();

    let nonce_before = eth_provider.transaction_count(eoa.evm_address().unwrap(), None).await.unwrap();

    // When
    eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Then
    let nonce_after = eth_provider.transaction_count(eoa.evm_address().unwrap(), None).await.unwrap();
    assert_eq!(nonce_before + U256::from(1), nonce_after);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_code(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana: Katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    let bytecode = eth_provider.get_code(counter_address, None).await.unwrap();

    // Then
    let counter_bytecode = <KakarotEvmContract as EvmContract>::load_contract_bytecode("Counter")
        .expect("Failed to load counter bytecode");
    let expected =
        counter_bytecode.deployed_bytecode.unwrap().bytecode.unwrap().object.into_bytes().unwrap().as_ref().to_vec();
    assert_eq!(bytecode, Bytes::from(expected));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs(#[future] katana: Katana, _setup: ()) {
    // Given
    let provider = katana.eth_provider();

    // When
    let logs = provider.get_logs(Filter::default()).await.expect("Failed to get logs");

    // Then
    let logs = match logs {
        FilterChanges::Logs(logs) => logs,
        _ => panic!("Expected logs"),
    };
    assert!(!logs.is_empty());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_code_empty(#[future] contract_empty: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana: Katana = contract_empty.0;

    let counter = contract_empty.1;
    let eth_provider = katana.eth_provider();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    let bytecode = eth_provider.get_code(counter_address, None).await.unwrap();

    // Then
    assert_eq!(bytecode, Bytes::default());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_code_no_contract(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let bytecode = eth_provider.get_code(Address::random(), None).await.unwrap();

    // Then
    assert_eq!(bytecode, Bytes::default());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_estimate_gas(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let eoa = counter.0.eoa();
    let eth_provider = counter.0.eth_provider();
    let counter = counter.1;

    let chain_id = eth_provider.chain_id().await.unwrap().unwrap_or_default();
    let counter_address: Felt252Wrapper = counter.evm_address.into();

    let request = TransactionRequest {
        from: Some(eoa.evm_address().unwrap()),
        to: Some(counter_address.try_into().unwrap()),
        input: TransactionInput { input: None, data: Some(Bytes::from_str("0x371303c0").unwrap()) }, // selector of "function inc()"
        chain_id: Some(chain_id.to::<u64>()),
        ..Default::default()
    };

    // When
    let estimate = eth_provider.estimate_gas(request, None).await.unwrap();

    // Then
    assert!(estimate > U256::from(0));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_fee_history(#[future] katana: Katana, _setup: ()) {
    // Retrieve the Ethereum provider from the Katana instance.
    let eth_provider = katana.eth_provider();

    // Retrieve the most recent block number.
    let newest_block = katana.most_recent_transaction().unwrap().block_number.unwrap();

    // To ensure that the range includes all mocked blocks.
    let block_count = u64::MAX;

    // Get the total number of blocks in the database.
    let nbr_blocks = katana.count_block();

    // Call the fee_history method of the Ethereum provider.
    let fee_history =
        eth_provider.fee_history(U64::from(block_count), BlockNumberOrTag::Number(newest_block), None).await.unwrap();

    // Verify that the length of the base_fee_per_gas list in the fee history is equal
    // to the total number of blocks plus one.
    assert_eq!(fee_history.base_fee_per_gas.len(), nbr_blocks + 1);

    // Verify that the length of the gas_used_ratio list in the fee history is equal
    // to the total number of blocks.
    assert_eq!(fee_history.gas_used_ratio.len(), nbr_blocks);

    // Verify that the oldest block in the fee history is equal to zero.
    assert_eq!(fee_history.oldest_block, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "hive")]
async fn test_predeploy_eoa(#[future] katana: Katana, _setup: ()) {
    use futures::future::join_all;
    use kakarot_rpc::eth_provider::constant::CHAIN_ID;
    use kakarot_rpc::test_utils::eoa::KakarotEOA;
    use reth_primitives::B256;
    use starknet::providers::Provider;

    // Given
    let eoa = katana.eoa();
    let eth_provider = katana.eth_provider();
    let starknet_provider = eth_provider.starknet_provider();
    let other_eoa_1 = KakarotEOA::new(B256::from_str(&format!("0x{:0>64}", "0abde1")).unwrap(), eth_provider.clone());
    let other_eoa_2 = KakarotEOA::new(B256::from_str(&format!("0x{:0>64}", "0abde2")).unwrap(), eth_provider.clone());
    let chain_id = starknet_provider.chain_id().await.unwrap();
    CHAIN_ID.set(chain_id).expect("Failed to set chain id");

    let evm_address = eoa.evm_address().unwrap();
    let balance_before = eth_provider.balance(eoa.evm_address().unwrap(), None).await.unwrap();
    eoa.transfer(other_eoa_1.evm_address().unwrap(), 1).await.expect("Failed to transfer funds to other eoa 1");
    // Sleep for 2 seconds to let the transaction pass
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    eoa.transfer(other_eoa_2.evm_address().unwrap(), 2).await.expect("Failed to transfer funds to other eoa 2");
    // Sleep for 2 seconds to let the transaction pass
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // When
    let t1 = tokio::task::spawn(async move {
        other_eoa_1.transfer(evm_address, 1).await.expect("Failed to transfer funds back to eoa");
    });
    let t2 = tokio::task::spawn(async move {
        other_eoa_2.transfer(evm_address, 1).await.expect("Failed to transfer funds back to eoa");
    });
    join_all([t1, t2]).await;

    // Then
    // Await all transactions to pass
    while starknet_provider.block_number().await.unwrap() < 6 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    let balance_after = eth_provider.balance(evm_address, None).await.unwrap();
    assert_eq!(balance_after, balance_before - U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_receipts(#[future] katana: Katana, _setup: ()) {
    // Given: Ethereum provider instance and the most recent transaction
    let eth_provider = katana.eth_provider();
    let transaction = katana.most_recent_transaction().unwrap();

    // Then: Retrieve receipts by block number
    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(
            transaction.block_number.unwrap(),
        ))))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipts.len(), 1);
    let receipt = receipts.first().unwrap();
    assert_eq!(receipt.transaction_index, transaction.transaction_index);
    assert_eq!(receipt.block_hash, transaction.block_hash);
    assert_eq!(receipt.block_number, transaction.block_number);

    // Then: Retrieve receipts by block hash
    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Hash(RpcBlockHash::from(transaction.block_hash.unwrap()))))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipts.len(), 1);
    let receipt = receipts.first().unwrap();
    assert_eq!(receipt.transaction_index, transaction.transaction_index);
    assert_eq!(receipt.block_hash, transaction.block_hash);
    assert_eq!(receipt.block_number, transaction.block_number);

    // Then: Attempt to retrieve receipts for a non-existing block
    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Hash(RpcBlockHash::from(B256::from(U256::from(0xc0fefe))))))
        .await
        .unwrap();
    assert!(receipts.is_none());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_starknet_block_id(#[future] katana: Katana, _setup: ()) {
    // Given: Ethereum provider instance and the most recent transaction
    let eth_provider = katana.eth_provider();
    let transaction = katana.most_recent_transaction().unwrap();

    // When: Convert block number identifier to StarkNet block identifier
    let block_id = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(transaction.block_number.unwrap()));
    let pending_starknet_block_id = eth_provider.to_starknet_block_id(block_id).await.unwrap();

    // When: Convert block hash identifier to StarkNet block identifier
    let some_block_hash = reth_rpc_types::BlockId::Hash(RpcBlockHash::from(transaction.block_hash.unwrap()));
    let some_starknet_block_hash = eth_provider.to_starknet_block_id(some_block_hash).await.unwrap();

    // When: Convert block tag identifier to StarkNet block identifier
    let pending_block_tag = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Pending);
    let pending_block_tag_starknet = eth_provider.to_starknet_block_id(pending_block_tag).await.unwrap();

    // When: Attempt to convert an unknown block number identifier to StarkNet block identifier
    let unknown_block_number = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(u64::MAX));
    let unknown_starknet_block_number = eth_provider.to_starknet_block_id(unknown_block_number).await;

    // Then: Ensure the converted StarkNet block identifiers match the expected values
    assert_eq!(pending_starknet_block_id, starknet::core::types::BlockId::Number(transaction.block_number.unwrap()));
    assert_eq!(
        some_starknet_block_hash,
        starknet::core::types::BlockId::Hash(
            FieldElement::from_bytes_be(
                &U256::from_be_slice(transaction.block_hash.unwrap().as_slice())
                    .wrapping_rem(STARKNET_MODULUS)
                    .to_be_bytes()
            )
            .unwrap()
        )
    );
    assert_eq!(pending_block_tag_starknet, starknet::core::types::BlockId::Tag(BlockTag::Pending));
    assert!(unknown_starknet_block_number.is_err());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875000000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Send the transaction
    let _ = eth_provider
        .send_raw_transaction(transaction_signed.envelope_encoded())
        .await
        .expect("failed to send transaction");

    // Retrieve the transaction from the database
    let tx: Option<StoredPendingTransaction> =
        eth_provider.database().get_one(None, None).await.expect("Failed to get transaction");

    // Assert that the number of retries is 0
    assert_eq!(tx.clone().unwrap().retries, 0);

    let tx = tx.unwrap().tx;

    // Assert the transaction hash and block number
    assert_eq!(tx.hash, transaction_signed.hash());
    assert!(tx.block_number.is_none());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_wrong_signature(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875000000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let mut transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Set an incorrect signature
    transaction_signed.signature = Signature::default();

    // Send the transaction
    let _ = eth_provider.send_raw_transaction(transaction_signed.envelope_encoded()).await;

    // Retrieve the transaction from the database
    let tx: Option<StoredPendingTransaction> =
        eth_provider.database().get_one(None, None).await.expect("Failed to get transaction");

    // Assert that no transaction is found
    assert!(tx.is_none());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_transaction_by_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    // Retrieve an instance of the Ethereum provider from the test environment
    let eth_provider = katana.eth_provider();

    // Retrieve the first transaction from the test environment
    let first_transaction = katana.first_transaction().unwrap();

    // Check if the first transaction is returned correctly by the `transaction_by_hash` method
    assert_eq!(eth_provider.transaction_by_hash(first_transaction.hash).await.unwrap().unwrap(), first_transaction);

    // Check if a non-existent transaction returns None
    assert!(eth_provider.transaction_by_hash(B256::random()).await.unwrap().is_none());

    // Generate a pending transaction to be stored in the pending transactions collection
    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id: 1,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875000000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Send the transaction
    let _ = eth_provider
        .send_raw_transaction(transaction_signed.envelope_encoded())
        .await
        .expect("failed to send transaction");

    // Retrieve the pending transaction from the database
    let mut stored_transaction: StoredPendingTransaction =
        eth_provider.database().get_one(None, None).await.expect("Failed to get transaction").unwrap();

    let tx = stored_transaction.clone().tx;

    // Check if the pending transaction is returned correctly by the `transaction_by_hash` method
    assert_eq!(eth_provider.transaction_by_hash(tx.hash).await.unwrap().unwrap(), tx);

    // Modify the block number of the pending transaction
    stored_transaction.tx.block_number = Some(1111);

    // Insert the transaction into the final transaction collection
    let filter = into_filter("tx.hash", &stored_transaction.tx.hash, HASH_HEX_STRING_LEN);
    eth_provider
        .database()
        .update_one::<StoredTransaction>(stored_transaction.tx.into(), filter, true)
        .await
        .expect("Failed to insert documents");

    // Check if the final transaction is returned correctly by the `transaction_by_hash` method
    assert_eq!(eth_provider.transaction_by_hash(tx.hash).await.unwrap().unwrap().block_number, Some(1111));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_transactions(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // Insert the first transaction into the pending transactions collection with 0 retry
    let transaction1 = katana.eoa().mock_transaction_with_nonce(0).expect("Failed to get mock transaction");
    eth_provider
        .database()
        .update_one::<StoredPendingTransaction>(
            transaction1.clone().into(),
            into_filter("tx.hash", &transaction1.hash, HASH_HEX_STRING_LEN),
            true,
        )
        .await
        .expect("Failed to insert pending transaction in database");

    // Insert the transaction into the pending transactions collection with TRANSACTION_MAX_RETRIES + 1 retry
    // Shouldn't be retried as it has reached the maximum number of retries
    let transaction2 = katana.eoa().mock_transaction_with_nonce(1).expect("Failed to get mock transaction");
    eth_provider
        .database()
        .update_one::<StoredPendingTransaction>(
            StoredPendingTransaction::new(transaction2.clone(), TRANSACTION_MAX_RETRIES + 1),
            into_filter("tx.hash", &transaction2.hash, HASH_HEX_STRING_LEN),
            true,
        )
        .await
        .expect("Failed to insert pending transaction in database");

    // Insert the transaction into both the mined transactions and pending transactions collections
    // Shouln't be retried as it has already been mined
    let transaction3 = katana.eoa().mock_transaction_with_nonce(2).expect("Failed to get mock transaction");
    eth_provider
        .database()
        .update_one::<StoredPendingTransaction>(
            transaction3.clone().into(),
            into_filter("tx.hash", &transaction3.clone().hash, HASH_HEX_STRING_LEN),
            true,
        )
        .await
        .expect("Failed to insert pending transaction in database");
    eth_provider
        .database()
        .update_one::<StoredTransaction>(
            transaction3.clone().into(),
            into_filter("tx.hash", &transaction3.hash, HASH_HEX_STRING_LEN),
            true,
        )
        .await
        .expect("Failed to insert transaction in mined collection");

    // Retrieve the retried transactions.
    let retried_transactions = eth_provider.retry_transactions().await.expect("Failed to retry transactions");

    // Assert that there is only one retried transaction.
    assert_eq!(retried_transactions.len(), 1);

    // Retrieve the pending transactions.
    let pending_transactions = eth_provider
        .database()
        .get::<StoredPendingTransaction>(None, None)
        .await
        .expect("Failed get pending transactions");

    // Ensure that the spurious transactions are dropped from the pending transactions collection
    assert_eq!(pending_transactions.len(), 1);

    // Ensure that the retry is incremented for the first transaction
    assert_eq!(pending_transactions.first().unwrap().retries, 1);

    // Ensure that the transaction1 is still in the pending transactions collection
    assert_eq!(pending_transactions.first().unwrap().tx, transaction1);
}
