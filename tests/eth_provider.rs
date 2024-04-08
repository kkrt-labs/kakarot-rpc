#![cfg(feature = "testing")]
use std::cmp::min;
use std::str::FromStr;

use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::test_utils::eoa::Eoa as _;
use kakarot_rpc::test_utils::evm_contract::EvmContract;
use kakarot_rpc::test_utils::fixtures::{counter, katana, setup};
use kakarot_rpc::test_utils::mongo::{BLOCK_HASH, BLOCK_NUMBER};
use kakarot_rpc::test_utils::{evm_contract::KakarotEvmContract, katana::Katana};
use reth_primitives::serde_helper::{JsonStorageKey, U64HexOrNumber};
use reth_primitives::{hex, Address, BlockNumberOrTag, Bytes, B256, U256, U64};
use reth_rpc_types::request::TransactionInput;
use reth_rpc_types::{RpcBlockHash, TransactionRequest};
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
    // The block number is 3 because this is what we set in the mocked mongo database.
    let expected = U64::from(BLOCK_NUMBER);
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
    // ASCII code for "kakatest" is 0x6b616b6174657374
    assert_eq!(chain_id, U64::from(0x6b616b6174657374u64));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_by_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let block = eth_provider.block_by_hash(*BLOCK_HASH, false).await.unwrap().unwrap();

    // Then
    assert_eq!(block.header.hash, Some(*BLOCK_HASH));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_by_number(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let block = eth_provider.block_by_number(BlockNumberOrTag::Number(BLOCK_NUMBER), false).await.unwrap().unwrap();

    // Then
    assert_eq!(block.header.number, Some(U256::from(BLOCK_NUMBER)));
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
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_transaction_count_by_number(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let count =
        eth_provider.block_transaction_count_by_number(BlockNumberOrTag::Number(BLOCK_NUMBER)).await.unwrap().unwrap();

    // Then
    assert_eq!(count, U256::from(3));
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
    // Given
    let eth_provider = katana.eth_provider();
    let newest_block = 3;
    let block_count = 100u64;

    // When
    let fee_history = eth_provider
        .fee_history(U64HexOrNumber::from(block_count), BlockNumberOrTag::Number(newest_block), None)
        .await
        .unwrap();

    // Then
    let actual_block_count = min(block_count, newest_block + 1);
    assert_eq!(fee_history.base_fee_per_gas.len(), actual_block_count as usize + 1);
    assert_eq!(fee_history.gas_used_ratio.len(), actual_block_count as usize);
    assert_eq!(fee_history.oldest_block, U256::ZERO);
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
    // Given
    let eth_provider = katana.eth_provider();

    // Then
    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(BLOCK_NUMBER))))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipts.len(), 3);
    let receipt = receipts.first().unwrap();
    assert_eq!(receipt.transaction_index, U64::ZERO);
    assert_eq!(receipt.transaction_hash.unwrap(), B256::ZERO);
    assert_eq!(receipt.block_hash.unwrap(), *BLOCK_HASH);
    assert_eq!(receipt.block_number.unwrap(), U256::from(BLOCK_NUMBER));

    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Hash(RpcBlockHash::from(*BLOCK_HASH))))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipts.len(), 3);
    let receipt = receipts.first().unwrap();
    assert_eq!(receipt.transaction_index, U64::ZERO);
    assert_eq!(receipt.transaction_hash.unwrap(), B256::ZERO);
    assert_eq!(receipt.block_hash.unwrap(), *BLOCK_HASH);
    assert_eq!(receipt.block_number.unwrap(), U256::from(BLOCK_NUMBER));

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
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let block_id = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(BLOCK_NUMBER));
    let pending_starknet_block_id = eth_provider.to_starknet_block_id(block_id).await.unwrap();

    let some_block_hash = reth_rpc_types::BlockId::Hash(RpcBlockHash::from(*BLOCK_HASH));
    let some_starknet_block_hash = eth_provider.to_starknet_block_id(some_block_hash).await.unwrap();

    let some_block_number = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(1));
    let some_starknet_block_number = eth_provider.to_starknet_block_id(some_block_number).await.unwrap();

    let unknown_block_number = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(u64::MAX));
    let unknown_starknet_block_number = eth_provider.to_starknet_block_id(unknown_block_number).await;

    // Then
    assert_eq!(pending_starknet_block_id, starknet::core::types::BlockId::Number(0x1234_u64));
    assert_eq!(some_starknet_block_hash, starknet::core::types::BlockId::Hash(FieldElement::from(0x1234_u64)));
    assert_eq!(some_starknet_block_number, starknet::core::types::BlockId::Tag(BlockTag::Pending));
    assert!(unknown_starknet_block_number.is_err());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    let data = hex!("b901f202f901ee05228459682f008459682f11830209bf8080b90195608060405234801561001057600080fd5b50610175806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c80630c49c36c14610030575b600080fd5b61003861004e565b604051610045919061011d565b60405180910390f35b60606020600052600f6020527f68656c6c6f2073746174656d696e64000000000000000000000000000000000060405260406000f35b600081519050919050565b600082825260208201905092915050565b60005b838110156100be5780820151818401526020810190506100a3565b838111156100cd576000848401525b50505050565b6000601f19601f8301169050919050565b60006100ef82610084565b6100f9818561008f565b93506101098185602086016100a0565b610112816100d3565b840191505092915050565b6000602082019050818103600083015261013781846100e4565b90509291505056fea264697066735822122051449585839a4ea5ac23cae4552ef8a96b64ff59d0668f76bfac3796b2bdbb3664736f6c63430008090033c080a0136ebffaa8fc8b9fda9124de9ccb0b1f64e90fbd44251b4c4ac2501e60b104f9a07eb2999eec6d185ef57e91ed099afb0a926c5b536f0155dd67e537c7476e1471");

    let res = eth_provider.send_raw_transaction(data.into()).await.expect("failed to send transaction");
}
