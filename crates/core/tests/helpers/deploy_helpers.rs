use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use bytes::BytesMut;
use dojo_test_utils::sequencer::TestSequencer;
use dotenv::dotenv;
use ethers::abi::{Abi, Tokenize};
use kakarot_rpc_core::client::constants::CHAIN_ID;
use reth_primitives::{
    sign_message, Address, Bytes, Transaction, TransactionKind, TransactionSigned, TxEip1559, H256, U256,
};
use serde::Deserialize;
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
use toml;
use url::Url;

use super::constants::FEE_TOKEN_ADDRESS;

macro_rules! root_project_path {
    ($relative_path:expr) => {{
        let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // this is assuming this code will always be ran in the context where the CARGO_MANIFEST_DIR
        // <system>/kakarot-rpc/crates/core
        // this is done so we can have more flexibility in filepaths the artifacts are located
        // so multiple crates can access them, if necessary
        let project_root = crate_root.parent().unwrap().parent().unwrap();
        let full_path = project_root.join($relative_path);
        full_path
    }};
}

#[derive(Debug, Deserialize)]
struct Profile {
    default: DefaultProfile,
}

#[derive(Debug, Deserialize)]
struct DefaultProfile {
    out: String,
}

#[derive(Debug, Deserialize)]
struct FoundryConfig {
    profile: Profile,
}

fn get_foundry_default_out(file_path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: FoundryConfig = toml::from_str(&contents)?;

    // Since 'out' is a field in 'Default' struct, we can access it directly
    Ok(config.profile.default.out)
}

// This assumes you are adding a solidity file in kakarot-rpc/solidity_contracts
// and ran `forge build --names --force`
pub fn get_contract(filename: &str) -> (Abi, ethers::types::Bytes) {
    let dot_sol = format!("{filename}.sol");
    let dot_json = format!("{filename}.json");
    let foundry_toml_path = root_project_path!("foundry.toml");
    let foundry_default_out = get_foundry_default_out(&foundry_toml_path).unwrap();
    let compiled_solidity_path = std::path::Path::new(&foundry_default_out).join(dot_sol).join(dot_json);
    let compiled_solidity_path_from_root = root_project_path!(&compiled_solidity_path);

    // Read the content of the file
    let contents = fs::read_to_string(compiled_solidity_path_from_root).expect("{filename} isn't compiled.");

    // Parse the entire JSON content into a Value
    let json: serde_json::Value = serde_json::from_str(&contents).unwrap();

    // Extract the `abi` field and parse it into the Abi type
    let abi: Abi = serde_json::from_value(json["abi"].clone()).unwrap();

    // Extract the `bytecode` field
    let bytecode_str = json["bytecode"]["object"].as_str().unwrap();
    let bytecode_vec = hex::decode(bytecode_str.trim_start_matches("0x")).unwrap();
    let bytecode = ethers::types::Bytes::from(bytecode_vec);

    (abi, bytecode)
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
        (None, false) => {
            panic!("Contract does not take arguments in its constructor, but passed user params are not empty.")
        }
        (None, true) => bytecode.clone(),
        (Some(constructor), _) => {
            let res = constructor.encode_input(bytecode.to_vec(), &params);
            res.unwrap().into()
        }
    }
}

// Constructs an ethereum transaction with the correct Kakarot chain ID, and default values
// for everything but nonce, to, and bytes
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

pub fn create_raw_ethereum_tx(selector: [u8; 4], eoa_secret: H256, to: Address, args: Vec<U256>, nonce: u64) -> Bytes {
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
    let signature =
        sign_message(eoa_secret, transaction.signature_hash()).expect("Signing of ethereum transaction failed.");

    let signed_transaction = TransactionSigned::from_transaction_and_signature(transaction, signature);
    let mut raw_tx = BytesMut::new(); // Create a new empty buffer

    signed_transaction.encode_enveloped(&mut raw_tx); // Encode the transaction into the buffer

    raw_tx.to_vec().into()
}

// Allows us to destructure the starknet katana receipt types in a more concise way
fn into_receipt(maybe_receipt: MaybePendingTransactionReceipt) -> Option<InvokeTransactionReceipt> {
    if let MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) = maybe_receipt {
        Some(receipt)
    } else {
        None
    }
}

/// Deploys an EVM contract and returns its ABI and list of field elements.
///
/// # Parameters
///
/// * `url`: The URL of the Ethereum node.
/// * `eth_contract`: The Ethereum contract to deploy.
/// * `constructor_args`: The arguments to pass to the contract's constructor.
///
/// # Returns
///
/// * `Result`: A result containing either the ABI and list of field elements, or an error if the
///   deployment failed.
async fn deploy_evm_contract<T: Tokenize>(
    sequencer_url: Url,
    eoa_account_starknet_address: FieldElement,
    eoa_secret: H256,
    contract_name: &str,
    constructor_args: T,
) -> Option<(Abi, Vec<FieldElement>)> {
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
        .expect("Deployment of ethereum contract failed.");

    let maybe_receipt = eoa_starknet_account
        .provider()
        .get_transaction_receipt(deployment_of_counter_evm_contract_result.transaction_hash)
        .await
        .unwrap();

    into_receipt(maybe_receipt).and_then(|InvokeTransactionReceipt { events, .. }| {
        events
            .iter()
            .find(|event| event.keys.contains(&get_selector_from_name("evm_contract_deployed").unwrap()))
            .map(|event| (abi, event.data.clone()))
    })
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
    let compiled_kakarot_path = root_project_path!(
        std::env::var("COMPILED_KAKAROT_PATH").expect("Expected a COMPILED_KAKAROT_PATH environment variable")
    );

    let paths = fs::read_dir(&compiled_kakarot_path)
        .unwrap_or_else(|_| panic!("Could not read directory: {}", compiled_kakarot_path.display()));

    let kakarot_compiled_contract_paths: Vec<_> = paths
        .filter_map(|entry| {
            let path = entry.expect("Failed to read directory entry").path();
            if path.is_dir() || path.extension().unwrap_or_default() != "json" { None } else { Some(path) }
        })
        .collect();

    assert!(!kakarot_compiled_contract_paths.is_empty(), "{} is empty.", compiled_kakarot_path.display());

    let mut class_hash: HashMap<String, FieldElement> = HashMap::new();
    for path in kakarot_compiled_contract_paths {
        let file = fs::File::open(&path).unwrap_or_else(|_| panic!("Failed to open file: {}", path.display()));
        let legacy_contract: LegacyContractClass = serde_json::from_reader(file)
            .unwrap_or_else(|_| panic!("Failed to deserialize contract from file: {}", path.display()));
        let contract_class = Arc::new(legacy_contract);

        let filename =
            path.file_stem().expect("File has no stem").to_str().expect("Cannot convert filename to string").to_owned();

        let res = account
            .declare_legacy(contract_class)
            .send()
            .await
            .unwrap_or_else(|_| panic!("Failed to declare {}", filename));

        class_hash.insert(filename, res.class_hash);
    }
    class_hash
}

async fn compute_starknet_address(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    contract_address: FieldElement,
    eoa_account_address: FieldElement,
) -> FieldElement {
    let call_get_starknet_address = FunctionCall {
        contract_address,
        entry_point_selector: get_selector_from_name("compute_starknet_address").unwrap(),
        calldata: vec![eoa_account_address],
    };

    let eoa_account_starknet_address_result =
        account.provider().call(call_get_starknet_address, BlockId::Tag(BlockTag::Latest)).await;

    *eoa_account_starknet_address_result.unwrap().first().unwrap()
}

async fn deploy_eoa(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    contract_address: FieldElement,
    eoa_account_address: FieldElement,
) {
    account
        .execute(vec![Call {
            calldata: vec![eoa_account_address],
            to: contract_address,
            selector: get_selector_from_name("deploy_externally_owned_account").unwrap(),
        }])
        .send()
        .await
        .expect("EOA deployment failed.");
}

async fn fund_eoa(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    eoa_account_starknet_address: FieldElement,
    amount: FieldElement,
    fee_token_address: FieldElement,
) {
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
}

async fn deploy_and_fund_eoa(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    contract_address: FieldElement,
    amount: FieldElement,
    eoa_account_address: FieldElement,
    fee_token_address: FieldElement,
) -> FieldElement {
    let eoa_account_starknet_address = compute_starknet_address(account, contract_address, eoa_account_address).await;
    deploy_eoa(account, contract_address, eoa_account_address).await;
    fund_eoa(account, eoa_account_starknet_address, amount, fee_token_address).await;

    eoa_account_starknet_address
}

async fn deploy_kakarot(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    class_hash: &HashMap<String, FieldElement>,
    fee_token_address: FieldElement,
) -> FieldElement {
    let kkrt_constructor_calldata = vec![
        account.address(),
        fee_token_address,
        *class_hash.get("contract_account").unwrap(),
        *class_hash.get("externally_owned_account").unwrap(),
        *class_hash.get("proxy").unwrap(),
    ];

    deploy_starknet_contract(account, class_hash.get("kakarot").unwrap(), kkrt_constructor_calldata)
        .await
        .expect("Failed to deploy Kakarot contract")
}

async fn deploy_blockhash_registry(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    class_hash: &HashMap<String, FieldElement>,
    kkrt_address: FieldElement,
) -> FieldElement {
    let blockhash_registry_calldata = vec![kkrt_address];

    deploy_starknet_contract(account, class_hash.get("blockhash_registry").unwrap(), blockhash_registry_calldata)
        .await
        .expect("Failed to deploy BlockhashRegistry contract")
}

async fn set_blockhash_registry(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    kkrt_address: FieldElement,
    blockhash_registry_addr: FieldElement,
) {
    let call_set_blockhash_registry = vec![Call {
        to: kkrt_address,
        selector: get_selector_from_name("set_blockhash_registry").unwrap(),
        calldata: vec![blockhash_registry_addr],
    }];

    account.execute(call_set_blockhash_registry).send().await.expect("`set_blockhash_registry` failed");
}

async fn deploy_kakarot_contracts(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    class_hash: &HashMap<String, FieldElement>,
    fee_token_address: FieldElement,
) -> HashMap<String, FieldElement> {
    let mut deployments: HashMap<String, FieldElement> = HashMap::new();

    let kkrt_address = deploy_kakarot(account, class_hash, fee_token_address).await;
    deployments.insert("kakarot".to_string(), kkrt_address);

    let blockhash_registry_addr = deploy_blockhash_registry(account, class_hash, kkrt_address).await;
    deployments.insert("blockhash_registry".to_string(), blockhash_registry_addr);

    set_blockhash_registry(account, kkrt_address, blockhash_registry_addr).await;

    deployments
}

pub struct DeployedKakarot {
    eoa_private_key: H256,
    pub kakarot: FieldElement,
    pub kakarot_proxy: FieldElement,
    pub eoa_starknet_address: FieldElement,
}

impl DeployedKakarot {
    // More delicate error handling here to enable explicit checking that certain conditions correctly
    // *fail* to deploy a contract
    pub async fn deploy_evm_contract<T: Tokenize>(
        &self,
        starknet_sequencer_url: Url,
        eth_contract: &str,
        constructor_args: T,
    ) -> Result<(Abi, Vec<FieldElement>), Box<dyn std::error::Error>> {
        deploy_evm_contract(
            starknet_sequencer_url,
            self.eoa_starknet_address,
            self.eoa_private_key,
            eth_contract,
            constructor_args,
        )
        .await
        .ok_or_else(|| "Evm contract deployment failed.".into())
    }
}

pub async fn deploy_kakarot_system(
    sequencer: &TestSequencer,
    kakarot_eoa_address: &str,
    eoa_private_key: H256,
    funding_amount: FieldElement,
) -> DeployedKakarot {
    dotenv().ok();

    let starknet_account = sequencer.account();
    let class_hash = declare_kakarot_contracts(&starknet_account).await;
    let kkrt_eoa_addr = FieldElement::from_hex_be(kakarot_eoa_address).unwrap();
    let fee_token_address = FieldElement::from_hex_be(FEE_TOKEN_ADDRESS).unwrap();
    let deployments = deploy_kakarot_contracts(&starknet_account, &class_hash, fee_token_address).await;
    let kkrt_address = deployments.get("kakarot").unwrap();
    let sn_eoa_address =
        deploy_and_fund_eoa(&starknet_account, *kkrt_address, funding_amount, kkrt_eoa_addr, fee_token_address).await;

    DeployedKakarot {
        eoa_private_key,
        kakarot: *kkrt_address,
        kakarot_proxy: *class_hash.get("proxy").unwrap(),
        eoa_starknet_address: sn_eoa_address,
    }
}
