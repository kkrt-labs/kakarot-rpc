use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use bytes::BytesMut;
use dojo_test_utils::sequencer::TestSequencer;
use ethers::abi::{Abi, Bytes as EthersBytes, JsonAbi};
use reth_primitives::{sign_message, Transaction, TransactionKind, TransactionSigned, TxEip1559, H256};
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

pub fn get_contract(filename: &str) -> (Abi, EthersBytes) {
    let path = format!("./tests/compiled_solidity/{filename}");
    let contents = fs::read_to_string(path).unwrap();
    let obj: JsonAbi = serde_json::from_str(&contents).unwrap();
    let JsonAbi::Object(obj) = obj else { panic!() };
    (serde_json::from_str(&serde_json::to_string(&obj.abi).unwrap()).unwrap(), obj.bytecode.unwrap().to_vec())
}

async fn deploy_kkrt_eth_contract(
    sequencer_url: Url,
    deployed_eao_starknet_account: FieldElement,
    eoa_secret: H256,
    contract_name: &str,
) -> Option<Vec<FieldElement>> {
    // need contract artifact
    // turns into the data field of a tx_payload
    // that gets signed and executed in the starknet eoa 'execute'/invoke
    // can first experiment with doing a call for `get_nonce`

    let signing_key = SigningKey::from_secret_scalar(FieldElement::from_byte_slice_be(eoa_secret.as_slice()).unwrap());

    let eoa_starknet_account = SingleOwnerAccount::new(
        JsonRpcClient::new(HttpTransport::new(sequencer_url)),
        LocalWallet::from_signing_key(signing_key),
        deployed_eao_starknet_account,
        chain_id::TESTNET,
    );

    // TODO: dehardcode
    let chain_id = 1263227476;
    let nonce = 0;
    let gas = 1000;

    let (_abi, contract_bytes) = get_contract(contract_name);
    let contract_bytes_reth: reth_primitives::Bytes = contract_bytes.into(); // convert Vec<u8> to reth_primitives::Bytes

    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce,
        max_priority_fee_per_gas: Default::default(), // U256 used for large numbers

        max_fee_per_gas: Default::default(), // U256 used for large numbers

        gas_limit: gas, // this may need to be converted to u64

        to: TransactionKind::Create,

        value: 0u128, // U256 used for large numbers

        input: contract_bytes_reth,

        access_list: Default::default(), // empty access list
    });

    let signature = sign_message(eoa_secret, transaction.signature_hash()).unwrap();

    let signed_transaction = TransactionSigned::from_transaction_and_signature(transaction, signature);
    let mut buffer = BytesMut::new(); // Create a new empty buffer

    let not_used_eoa_account_fields = FieldElement::from_hex_be("0xDEAD").unwrap();
    signed_transaction.encode_enveloped(&mut buffer); // Encode the transaction into the buffer
    let bytes_vec = buffer.to_vec(); // Get Vec<u8> from Bytes

    let field_elements: Vec<FieldElement> = bytes_vec
        .into_iter()
        .map(|byte| {
            // Convert each byte to a field element
            FieldElement::from(byte)
        })
        .collect();

    let deployment_of_counter_evm_contract_result = eoa_starknet_account
        .execute(vec![Call {
            calldata: field_elements,
            to: not_used_eoa_account_fields,
            selector: not_used_eoa_account_fields,
        }])
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

    let result = factory.deploy(constructor_calldata.clone(), FieldElement::ZERO, false).send().await.unwrap();

    let contract_address =
        get_contract_address(FieldElement::ZERO, *class_hash, &constructor_calldata.clone(), FieldElement::ZERO);

    // TODO: add a check here
    let _receipt = account.provider().get_transaction_receipt(result.transaction_hash).await.unwrap();

    Ok(contract_address)
}

async fn declare_kkrt_contracts(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
) -> HashMap<String, FieldElement> {
    let paths = fs::read_dir("tests/compiled_kkrt").expect("Could not read directory");

    let kkrt_compiled_contracts: Vec<_> = paths
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
    for path in kkrt_compiled_contracts {
        let file = fs::File::open(&path).unwrap();
        let legacy_contract: LegacyContractClass = serde_json::from_reader(file).unwrap();
        let contract_class = Arc::new(legacy_contract);

        let res = account.declare_legacy(contract_class).send().await.unwrap();
        // TODO add assert
        let _receipt = account.provider().get_transaction_receipt(res.transaction_hash).await.unwrap();

        let filename =
            path.file_stem().expect("File has no stem").to_str().expect("Cannot convert filename to string").to_owned();

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

    let deployment_of_eoa_account_result = account
        .execute(vec![Call {
            calldata: vec![eoa_account_address],
            // devnet UDC address
            to: kkrt_address,
            selector: get_selector_from_name("deploy_externally_owned_account").unwrap(),
        }])
        .send()
        .await
        .unwrap();

    // TODO: add check here
    let _deployment_of_eoa_account_result_receipt =
        account.provider().get_transaction_receipt(deployment_of_eoa_account_result.transaction_hash).await.unwrap();

    let eoa_account_starknet_address = *eoa_account_starknet_address_result.unwrap().first().unwrap();

    let amount_high = FieldElement::from_dec_str("0").unwrap();
    let transfer_calldata = vec![eoa_account_starknet_address, amount, amount_high];

    let transfer_res = account
        .execute(vec![Call {
            calldata: transfer_calldata,
            // eth fee addr
            to: fee_token_address,
            selector: get_selector_from_name("transfer").unwrap(),
        }])
        .send()
        .await
        .unwrap();

    // TODO: add checks like the python deploy script has
    let _transfer_receipt = account.provider().get_transaction_receipt(transfer_res.transaction_hash).await.unwrap();

    let call_get_balance_of_starknet_address_of_eoa = FunctionCall {
        contract_address: fee_token_address,
        entry_point_selector: get_selector_from_name("balanceOf").unwrap(),
        calldata: vec![eoa_account_starknet_address],
    };

    let _balance_of_starknet_address_of_eoa_result =
        account.provider().call(call_get_balance_of_starknet_address_of_eoa, BlockId::Tag(BlockTag::Latest)).await;

    let call_get_nonce_of_starknet_address_of_eoa = FunctionCall {
        contract_address: eoa_account_starknet_address,
        entry_point_selector: get_selector_from_name("get_nonce").unwrap(),
        calldata: vec![],
    };

    // TODO: use to derive nonce when we use this account to deploy
    let _nonce_of_starknet_address_of_eoa_result =
        account.provider().call(call_get_nonce_of_starknet_address_of_eoa, BlockId::Tag(BlockTag::Latest)).await;

    eoa_account_starknet_address
}

async fn deploy_kkrt_contracts(
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

    // TODO: add a check here
    let result = account.execute(call_set_blockhash_registry).send().await.unwrap();
    let _receipt = account.provider().get_transaction_receipt(result.transaction_hash).await.unwrap();
    deployments
}

// TODO: add more meaningful type signature
pub async fn init_kkrt_state(
    sequencer: &TestSequencer,
    kkrt_eoa_address: &str,
    kkrt_eoa_private: H256,
    funding_amount: FieldElement,
    eth_contract: &str,
) -> (FieldElement, FieldElement, FieldElement, Vec<FieldElement>) {
    let account = sequencer.account();
    let class_hash = declare_kkrt_contracts(&account).await;
    let kkrt_eoa_addr = FieldElement::from_hex_be(kkrt_eoa_address).unwrap();
    let fee_token_address =
        FieldElement::from_hex_be("0x49D36570D4E46F48E99674BD3FCC84644DDD6B96F7C741B1562B82F9E004DC7").unwrap();
    let deployments = deploy_kkrt_contracts(&account, &class_hash, fee_token_address).await;
    let kkrt_address = deployments.get("kakarot").unwrap();
    let sn_eoa_address =
        deploy_and_fund_eoa(&account, *kkrt_address, funding_amount, kkrt_eoa_addr, fee_token_address).await;
    // TODO: make better parameterization/env reading

    let deploy_event = deploy_kkrt_eth_contract(sequencer.url(), sn_eoa_address, kkrt_eoa_private, eth_contract).await;
    (*kkrt_address, *class_hash.get("proxy").unwrap(), sn_eoa_address, deploy_event.unwrap())
}
