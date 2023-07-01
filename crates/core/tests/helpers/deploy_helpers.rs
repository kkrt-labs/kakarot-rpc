use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use bytes::BytesMut;
use dojo_test_utils::sequencer::TestSequencer;
use ethers::abi::{Abi, JsonAbi, Tokenize};
use kakarot_rpc_core::client::constants::CHAIN_ID;
use reth_primitives::{
    sign_message, Address, Bytes, Transaction, TransactionKind, TransactionSigned, TxEip1559, H256, U256,
};
use starknet::accounts::{Account, Call, ConnectedAccount, SingleOwnerAccount};
use starknet::contract::ContractFactory;
use starknet::core::chain_id;
use starknet::core::types::contract::legacy::LegacyContractClass;
use starknet::core::types::{
    BlockId, BlockTag, FieldElement, FunctionCall, InvokeTransactionReceipt, MaybePendingTransactionReceipt,
    TransactionReceipt,
};
use starknet::core::utils::{get_contract_address, get_selector_from_name};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider};
use starknet::signers::{LocalWallet, SigningKey};
use url::Url;

use super::constants::{COMPILED_KAKAROT_PATH, COMPILED_SOLIDITY_PATH, FEE_TOKEN_ADDRESS};

pub fn get_contract(filename: &str) -> (Abi, ethers::types::Bytes) {
    let path = format!("{COMPILED_SOLIDITY_PATH}/{filename}");
    let contents = fs::read_to_string(path).unwrap();
    let obj: JsonAbi = serde_json::from_str(&contents).unwrap();
    let JsonAbi::Object(obj) = obj else { panic!() };
    (serde_json::from_str(&serde_json::to_string(&obj.abi).unwrap()).unwrap(), obj.bytecode.unwrap())
}

// extracted/simplified logic from ethers https://github.com/gakonst/ethers-rs/blob/master/ethers-contract/src/factory.rs#L385
// otherwise we would have to mock an eth client in order to access the logic
pub fn encode_contract<T: Tokenize>(
    abi: &Abi,
    bytecode: &ethers::types::Bytes,
    constructor_args: T,
) -> ethers::types::Bytes {
    let params = constructor_args.into_tokens();
    match (abi.constructor(), params.is_empty()) {
        (None, false) => panic!("No constructor in ABI."),
        (None, true) => bytecode.clone(),
        (Some(constructor), _) => {
            let res = constructor.encode_input(bytecode.to_vec(), &params);
            res.unwrap().into()
        }
    }
}

// the ethereum transaction you know and love,
// with defaults where we can, and the kakarot chain_id set
pub fn to_kakarot_transaction(nonce: u64, to: TransactionKind, input: Bytes) -> Transaction {
    Transaction::Eip1559(TxEip1559 {
        chain_id: CHAIN_ID,
        nonce,
        max_priority_fee_per_gas: Default::default(),
        max_fee_per_gas: Default::default(),
        gas_limit: Default::default(),
        to,
        value: Default::default(),
        input,
        access_list: Default::default(),
    })
}

pub fn create_raw_tx(selector: [u8; 4], eoa_secret: H256, to: Address, args: Vec<U256>, nonce: u64) -> Bytes {
    // Start with the function selector
    // Append each argument
    let mut data: Vec<u8> = selector.to_vec();

    for arg in args {
        // Ethereum uses big-endian encoding
        let arg_bytes: [u8; 32] = arg.to_be_bytes();
        data.extend_from_slice(&arg_bytes);
    }

    // Create a transaction object
    let transaction = to_kakarot_transaction(nonce, TransactionKind::Call(to), data.into());
    let signature = sign_message(eoa_secret, transaction.signature_hash()).unwrap();

    let signed_transaction = TransactionSigned::from_transaction_and_signature(transaction, signature);
    let mut raw_tx = BytesMut::new(); // Create a new empty buffer

    signed_transaction.encode_enveloped(&mut raw_tx); // Encode the transaction into the buffer

    raw_tx.to_vec().into()
}

async fn deploy_evm_contract<T: Tokenize>(
    sequencer_url: Url,
    eoa_account_starknet_address: FieldElement,
    eoa_secret: H256,
    contract_name: &str,
    constructor_args: T,
) -> Option<Vec<FieldElement>> {
    // This a made up signing key so we can reuse starknet-rs abstractions
    // to see the flow of how kakarot-rpc handles eth payloads -> starknet txns
    // see ./crates/core/src/client.rs::send_transaction
    let signing_key = SigningKey::from_secret_scalar(FieldElement::ZERO);

    let eoa_starknet_account = SingleOwnerAccount::new(
        JsonRpcClient::new(HttpTransport::new(sequencer_url)),
        LocalWallet::from_signing_key(signing_key),
        eoa_account_starknet_address,
        chain_id::TESTNET,
    );

    let (abi, contract_bytes) = get_contract(contract_name);
    let contract_bytes = encode_contract(&abi, &contract_bytes, constructor_args);

    let transaction = to_kakarot_transaction(0, TransactionKind::Create, contract_bytes.to_vec().into());
    let signature = sign_message(eoa_secret, transaction.signature_hash()).unwrap();
    let signed_transaction = TransactionSigned::from_transaction_and_signature(transaction, signature);

    let mut buffer = BytesMut::new(); // Create a new empty buffer
    signed_transaction.encode_enveloped(&mut buffer); // Encode the transaction into the buffer
    let bytes_vec = buffer.to_vec(); // Get Vec<u8> from Bytes

    let field_elements: Vec<FieldElement> = bytes_vec
        .into_iter()
        .map(|byte| {
            // Convert each byte to a field element
            FieldElement::from(byte)
        })
        .collect();

    let unused_eoa_field = FieldElement::from_hex_be("0xDEAD").unwrap();
    let deployment_of_counter_evm_contract_result = eoa_starknet_account
        .execute(vec![Call { calldata: field_elements, to: unused_eoa_field, selector: unused_eoa_field }])
        .send()
        .await
        .unwrap();

    let deployment_of_counter_evm_contract_result_receipt = eoa_starknet_account
        .provider()
        .get_transaction_receipt(deployment_of_counter_evm_contract_result.transaction_hash)
        .await
        .unwrap();

    if let MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(InvokeTransactionReceipt {
        events,
        ..
    })) = deployment_of_counter_evm_contract_result_receipt
    {
        events.iter().find_map(|event| {
            if event.keys.contains(&get_selector_from_name("evm_contract_deployed").unwrap()) {
                Some(event.data.clone())
            } else {
                None
            }
        })
    } else {
        None
    }
}

async fn deploy_starknet_contract(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    class_hash: &FieldElement,
    constructor_calldata: Vec<FieldElement>,
) -> Result<FieldElement, Box<dyn std::error::Error>> {
    let factory = ContractFactory::new(*class_hash, account);

    factory.deploy(constructor_calldata.clone(), FieldElement::ZERO, false).send().await?;

    let contract_address =
        get_contract_address(FieldElement::ZERO, *class_hash, &constructor_calldata.clone(), FieldElement::ZERO);

    Ok(contract_address)
}

async fn declare_kakarot_contracts(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
) -> HashMap<String, FieldElement> {
    let paths = fs::read_dir(COMPILED_KAKAROT_PATH).expect("Could not read directory");

    let kakarot_compiled_contract_paths: Vec<_> = paths
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().unwrap_or_default() != "json" {
                return None;
            }
            Some(path)
        })
        .collect();

    let mut class_hash: HashMap<String, FieldElement> = HashMap::new();
    for path in kakarot_compiled_contract_paths {
        let file = fs::File::open(&path).unwrap();
        let legacy_contract: LegacyContractClass = serde_json::from_reader(file).unwrap();
        let contract_class = Arc::new(legacy_contract);

        let filename =
            path.file_stem().expect("File has no stem").to_str().expect("Cannot convert filename to string").to_owned();

        let res = account
            .declare_legacy(contract_class)
            .send()
            .await
            .unwrap_or_else(|_| panic!("failed to declare {}", filename));

        class_hash.insert(filename, res.class_hash);
    }
    class_hash
}

async fn deploy_and_fund_eoa(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    kkrt_address: FieldElement,
    amount: FieldElement,
    eoa_account_address: FieldElement,
    fee_token_address: FieldElement,
) -> FieldElement {
    let call_get_starknet_address = FunctionCall {
        contract_address: kkrt_address,
        entry_point_selector: get_selector_from_name("compute_starknet_address").unwrap(),
        calldata: vec![eoa_account_address],
    };

    let eoa_account_starknet_address_result =
        account.provider().call(call_get_starknet_address, BlockId::Tag(BlockTag::Latest)).await;

    account
        .execute(vec![Call {
            calldata: vec![eoa_account_address],
            to: kkrt_address,
            selector: get_selector_from_name("deploy_externally_owned_account").unwrap(),
        }])
        .send()
        .await
        .expect("EOA deployment failed.");

    let eoa_account_starknet_address = *eoa_account_starknet_address_result.unwrap().first().unwrap();

    let amount_high = FieldElement::ZERO;
    let transfer_calldata = vec![eoa_account_starknet_address, amount, amount_high];

    account
        .execute(vec![Call {
            calldata: transfer_calldata,
            // eth fee addr
            to: fee_token_address,
            selector: get_selector_from_name("transfer").unwrap(),
        }])
        .send()
        .await
        .expect("Funding test eth account failed.");

    eoa_account_starknet_address
}

async fn deploy_kakarot_contracts(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    class_hash: &HashMap<String, FieldElement>,
    fee_token_address: FieldElement,
) -> HashMap<String, FieldElement> {
    let mut deployments: HashMap<String, FieldElement> = HashMap::new();

    let kkrt_constructor_calldata = vec![
        account.address(),
        fee_token_address,
        *class_hash.get("contract_account").unwrap(),
        *class_hash.get("externally_owned_account").unwrap(),
        *class_hash.get("proxy").unwrap(),
    ];

    let kkrt_res = deploy_starknet_contract(account, class_hash.get("kakarot").unwrap(), kkrt_constructor_calldata);

    deployments.insert("kakarot".to_string(), kkrt_res.await.unwrap());

    let kkrt_address = *deployments.get("kakarot").unwrap();

    let blockhash_registry_calldata = vec![kkrt_address];

    let blockhash_registry_res =
        deploy_starknet_contract(account, class_hash.get("blockhash_registry").unwrap(), blockhash_registry_calldata);

    deployments.insert("blockhash_registry".to_string(), blockhash_registry_res.await.unwrap());

    let blockhash_registry_addr = *deployments.get("blockhash_registry").unwrap();

    let call_set_blockhash_registry = vec![Call {
        to: kkrt_address,
        selector: get_selector_from_name("set_blockhash_registry").unwrap(),
        calldata: vec![blockhash_registry_addr],
    }];

    account.execute(call_set_blockhash_registry).send().await.expect("`set_blockhash_registry` failed");

    deployments
}

pub async fn init_kkrt_state<T: Tokenize>(
    sequencer: &TestSequencer,
    kkrt_eoa_address: &str,
    eoa_ethereum_private_key: H256,
    funding_amount: FieldElement,
    eth_contract: &str,
    constructor_args: T,
) -> (FieldElement, FieldElement, FieldElement, Vec<FieldElement>) {
    let account = sequencer.account();
    let class_hash = declare_kakarot_contracts(&account).await;
    let kkrt_eoa_addr = FieldElement::from_hex_be(kkrt_eoa_address).unwrap();
    let fee_token_address = FieldElement::from_hex_be(FEE_TOKEN_ADDRESS).unwrap();
    let deployments = deploy_kakarot_contracts(&account, &class_hash, fee_token_address).await;
    let kkrt_address = deployments.get("kakarot").unwrap();
    let sn_eoa_address =
        deploy_and_fund_eoa(&account, *kkrt_address, funding_amount, kkrt_eoa_addr, fee_token_address).await;

    let deploy_event =
        deploy_evm_contract(sequencer.url(), sn_eoa_address, eoa_ethereum_private_key, eth_contract, constructor_args)
            .await;
    (*kkrt_address, *class_hash.get("proxy").unwrap(), sn_eoa_address, deploy_event.unwrap())
}
