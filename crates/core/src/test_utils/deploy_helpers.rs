use std::collections::HashMap;
use std::fs::{self};
use std::path::PathBuf;
use std::sync::Arc;

use bytes::BytesMut;
use dojo_test_utils::sequencer::{Environment, SequencerConfig, StarknetConfig, TestSequencer};
use dotenv::dotenv;
use ethers::abi::{Abi, Tokenize};
use ethers::signers::{LocalWallet as EthersLocalWallet, Signer};
use ethers_solc::artifacts::CompactContractBytecode;
use foundry_config::utils::{find_project_root_path, load_config};
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

use crate::client::api::KakarotStarknetApi;
use crate::client::config::{Network, StarknetConfig as StarknetClientConfig};
use crate::client::constants::{CHAIN_ID, STARKNET_NATIVE_TOKEN};
use crate::client::KakarotClient;
use crate::contracts::kakarot::KakarotContract;
use crate::models::felt::Felt252Wrapper;
use crate::test_utils::constants::EOA_WALLET;

/// Macro to find the root path of the project.
///
/// This macro utilizes the `find_project_root_path` function from the `utils` module.
/// This function works by identifying the root directory of the current git repository.
/// It starts at the current working directory and traverses up the directory tree until it finds a
/// directory containing a `.git` folder. If no such directory is found, it uses the current
/// directory as the root.
///
/// After determining the project root, the macro creates a new path by joining the given relative
/// path with the found project root path.
///
/// The relative path must be specified as a string literal argument to the macro.
///
/// # Examples
///
/// ```ignore
/// let full_path = root_project_path!("src/main.rs");
/// println!("Full path to main.rs: {:?}", full_path);
/// ```
///
/// # Panics
///
/// This macro will panic if it fails to find the root path of the project or if the root path
/// cannot be represented as a UTF-8 string.
macro_rules! root_project_path {
    ($relative_path:expr) => {{
        let project_root_buf = find_project_root_path(None).unwrap();
        let project_root = project_root_buf.to_str().unwrap();
        let full_path = std::path::Path::new(project_root).join($relative_path);
        full_path
    }};
}

/// Returns the abi for a compact contract bytecode
pub fn get_contract_abi(contract: &CompactContractBytecode) -> Abi {
    contract.abi.as_ref().unwrap().to_owned()
}

/// Returns the bytecode for a compact contract bytecode
pub fn get_contract_bytecode(contract: &CompactContractBytecode) -> ethers::types::Bytes {
    contract.bytecode.as_ref().unwrap().object.as_bytes().unwrap().to_owned()
}

/// Returns the deployed bytecode for a compact contract bytecode
pub fn get_contract_deployed_bytecode(contract: CompactContractBytecode) -> ethers::types::Bytes {
    contract.deployed_bytecode.unwrap().bytecode.unwrap().object.as_bytes().unwrap().to_owned()
}

/// Loads and parses a compiled Solidity contract.
///
/// This function assumes that a Solidity source file has been added to the `solidity_contracts`
/// directory in the `lib/kakarot/tests/integration/solidity_contracts` path, and has been compiled
/// with the `forge build` command. It loads the resulting JSON artifact and returns a
/// CompactContractBytecode
pub fn get_contract(filename: &str) -> CompactContractBytecode {
    let dot_sol = format!("{filename}.sol");
    let dot_json = format!("{filename}.json");

    let foundry_default_out = load_config().out;
    let compiled_solidity_path = std::path::Path::new(&foundry_default_out).join(dot_sol).join(dot_json);
    let compiled_solidity_path_from_root = root_project_path!(&compiled_solidity_path);

    // Read the content of the file
    let contents = fs::read_to_string(compiled_solidity_path_from_root).unwrap_or_else(|_| {
        panic!("Could not read file: {}. please run `make setup` to ensure solidity files are compiled", filename)
    });

    serde_json::from_str(&contents).unwrap()
}

/// Encodes a contract's bytecode and constructor arguments into deployable bytecode.
///
/// This function is based on logic extracted and simplified from the `ethers-rs` crate (see
/// https://github.com/gakonst/ethers-rs/blob/master/ethers-contract/src/factory.rs#L385). This has
/// been done to avoid the need to mock an Ethereum client just to access this functionality.
///
/// # Panics
///
/// This function will panic if the contract's ABI does not define a constructor but constructor
/// arguments are provided, or if an error occurs while encoding the constructor's input.
///
/// # Type parameters
///
/// `T` - A type which can be tokenized into constructor arguments.
///
/// # Example
///
/// ```no_run
/// # use kakarot_rpc_core::test_utils::deploy_helpers::{get_contract, encode_contract, get_contract_abi, get_contract_bytecode};
/// # use ethers::abi::Abi;
/// let contract = get_contract("MyContract");
/// let abi = get_contract_abi(&contract);
/// let bytecode = get_contract_bytecode(&contract);
/// let constructor_args = (1, 42);
/// let deploy_bytecode = encode_contract(&abi, &bytecode, constructor_args);
/// ```
///
/// This example assumes that the `MyContract` contract takes a string and an integer in its
/// constructor.
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

/// Constructs a Kakarot transaction based on given parameters.
///
/// This function creates an EIP-1559 transaction with certain fields set according to the function
/// parameters and the others set to their default values.
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

/// Constructs and signs a raw Ethereum transaction based on given parameters.
///
/// This function creates a transaction which calls a contract function with provided arguments.
/// The transaction is signed using the provided EOA secret.
pub fn create_raw_ethereum_tx(
    selector: [u8; 4],
    eoa_secret_key: H256,
    to: Address,
    args: Vec<U256>,
    nonce: u64,
) -> Bytes {
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
        sign_message(eoa_secret_key, transaction.signature_hash()).expect("Signing of ethereum transaction failed.");

    let signed_transaction = TransactionSigned::from_transaction_and_signature(transaction, signature);
    let mut raw_tx = BytesMut::new(); // Create a new empty buffer

    signed_transaction.encode_enveloped(&mut raw_tx); // Encode the transaction into the buffer

    raw_tx.to_vec().into()
}

/// Allows us to destructure the starknet katana receipt types in a more concise way
fn into_receipt(maybe_receipt: MaybePendingTransactionReceipt) -> Option<InvokeTransactionReceipt> {
    if let MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) = maybe_receipt {
        Some(receipt)
    } else {
        None
    }
}

/// Deploys an EVM contract and returns its ABI and list of two field elements
/// the first being the FieldElement that represents the ethereum address of the deployed contract.
/// the second being the Field Element that's the underpinning starknet contract address
async fn deploy_evm_contract<T: Tokenize>(
    sequencer_url: Url,
    eoa_account_starknet_address: FieldElement,
    eoa_secret_key: H256,
    contract_name: &str,
    constructor_args: T,
) -> Option<(Abi, ContractAddresses)> {
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

    let contract = get_contract(contract_name);
    let abi = get_contract_abi(&contract);
    let contract_bytes = get_contract_bytecode(&contract);
    let contract_bytes = encode_contract(&abi, &contract_bytes, constructor_args);
    let nonce = eoa_starknet_account.get_nonce().await.unwrap();
    let transaction =
        to_kakarot_transaction(nonce.try_into().unwrap(), TransactionKind::Create, contract_bytes.to_vec().into());
    let signature = sign_message(eoa_secret_key, transaction.signature_hash()).unwrap();
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

    let unused_eoa_field = FieldElement::ZERO;
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
        events.iter().find(|event| event.keys.contains(&get_selector_from_name("evm_contract_deployed").unwrap())).map(
            |event| {
                (
                    abi,
                    ContractAddresses {
                        eth_address: {
                            let evm_address: Felt252Wrapper = event.data[0].into();
                            evm_address.try_into().unwrap()
                        },
                        starknet_address: event.data[1],
                    },
                )
            },
        )
    })
}

/// Asynchronously deploys a Starknet contract to the network using the provided account and
/// contract parameters.
///
/// This function uses a `ContractFactory` to prepare a deployment of a Starknet contract, and sends
/// the deployment transaction to the network. It then computes and returns the address at which the
/// contract will be deployed.
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

/// Asynchronously declares a set of Kakarot contracts on the network using the provided account.
///
/// This function reads compiled Kakarot contract files from a directory specified by the
/// `COMPILED_KAKAROT_PATH` environment variable. Each file is deserialized into a
/// `LegacyContractClass` object and declared on the network via the provided account.
///
/// After successfully declaring each contract, the function stores the class hash of the contract
/// into a HashMap with the contract name as the key.
///
/// # Panics
///
/// This function will panic if:
/// * The `COMPILED_KAKAROT_PATH` environment variable is not set.
/// * The directory specified by `COMPILED_KAKAROT_PATH` cannot be read.
/// * The directory specified by `COMPILED_KAKAROT_PATH` is empty.
/// * A contract file cannot be opened or deserialized.
/// * The contract declaration fails on the network.
///
/// This example declares all Kakarot contracts in the directory specified by
/// `COMPILED_KAKAROT_PATH`, and prints the name and class hash of each contract.
async fn declare_kakarot_contracts(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
) -> HashMap<String, FieldElement> {
    let kakarot_compiled_contract_paths = compiled_kakarot_paths();

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
    let call_compute_starknet_address = FunctionCall {
        contract_address,
        entry_point_selector: get_selector_from_name("compute_starknet_address").unwrap(),
        calldata: vec![eoa_account_address],
    };

    let eoa_account_starknet_address_result =
        account.provider().call(call_compute_starknet_address, BlockId::Tag(BlockTag::Latest)).await;

    *eoa_account_starknet_address_result.unwrap().first().unwrap()
}

pub fn compute_kakarot_contracts_class_hash() -> Vec<(String, FieldElement)> {
    // Get the compiled Kakarot contracts directory path.
    let kakarot_compiled_contract_paths = compiled_kakarot_paths();

    // Deserialize each contract file into a `LegacyContractClass` object.
    // Compute the class hash of each contract.
    kakarot_compiled_contract_paths
        .iter()
        .map(|path| {
            let file = fs::File::open(path).unwrap_or_else(|_| panic!("Failed to open file: {}", path.display()));
            let contract_class: LegacyContractClass = serde_json::from_reader(file)
                .unwrap_or_else(|_| panic!("Failed to deserialize contract from file: {}", path.display()));

            let filename = path
                .file_stem()
                .expect("File has no stem")
                .to_str()
                .expect("Cannot convert filename to string")
                .to_owned();

            // Compute the class hash
            (filename, contract_class.class_hash().expect("Failed to compute class hash"))
        })
        .collect()
}

fn compiled_kakarot_paths() -> Vec<PathBuf> {
    dotenv().ok();
    let compiled_kakarot_path = root_project_path!(std::env::var("COMPILED_KAKAROT_PATH").expect(
        "Expected a COMPILED_KAKAROT_PATH environment variable, set up your .env file or use \
         `./scripts/make_with_env.sh test`"
    ));

    let paths = fs::read_dir(&compiled_kakarot_path)
        .unwrap_or_else(|_| panic!("Could not read directory: {}", compiled_kakarot_path.display()));

    let kakarot_compiled_contract_paths: Vec<_> = paths
        .filter_map(|entry| {
            let path = entry.expect("Failed to read directory entry").path();
            if path.is_dir() || path.extension().unwrap_or_default() != "json" { None } else { Some(path) }
        })
        .collect();

    assert!(
        !kakarot_compiled_contract_paths.is_empty(),
        "{} is empty, please run `make setup` to ensure that Kakarot evm is built.",
        compiled_kakarot_path.display()
    );

    kakarot_compiled_contract_paths
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

/// Asynchronously deploys an Externally Owned Account (EOA) to the network and funds it.
///
/// This function first computes the Starknet address of the EOA to be deployed using the provided
/// account, contract address, and EOA account address. Then, it deploys the EOA to the network and
/// funds it with the specified amount of fee token.
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

/// Structure representing a deployed Kakarot system, containing key details of the system.
///
/// This includes the private key and address of the Externally Owned Account (EOA), the Starknet
/// addresses of the kakarot and kakarot_proxy contracts, and the Starknet address of the EOA.
pub struct DeployedKakarot {
    pub eoa_private_key: H256,
    pub kakarot_address: FieldElement,
    pub proxy_class_hash: FieldElement,
    pub contract_account_class_hash: FieldElement,
    pub eoa_addresses: ContractAddresses,
}

pub struct ContractAddresses {
    pub eth_address: Address,
    pub starknet_address: FieldElement,
}

impl DeployedKakarot {
    /// Asynchronously deploys an EVM contract.
    ///
    /// This function deploys an EVM contract to the Starknet network by calling the
    /// `deploy_evm_contract` function. It also wraps around the result to provide error
    /// handling capabilities. It returns an error when the deployment fails.
    ///
    /// # Arguments
    ///
    /// * `starknet_sequencer_url` - A `Url` indicating the URL of the Starknet sequencer.
    ///
    /// * `eth_contract` - A string representing the name of the Ethereum contract to deploy.
    ///
    /// * `constructor_args` - A generic argument that implements the `Tokenize` trait, representing
    ///   the constructor arguments.
    ///
    /// # Returns
    ///
    /// A `Result` that holds the `Abi` of the contract and a vector of `FieldElement` representing
    /// the contract deployment, or an error of type `Box<dyn std::error::Error>` when the
    /// deployment fails.
    pub async fn deploy_evm_contract<T: Tokenize>(
        &self,
        starknet_sequencer_url: Url,
        eth_contract: &str,
        constructor_args: T,
    ) -> Result<(Abi, ContractAddresses), Box<dyn std::error::Error>> {
        deploy_evm_contract(
            starknet_sequencer_url,
            self.eoa_addresses.starknet_address,
            self.eoa_private_key,
            eth_contract,
            constructor_args,
        )
        .await
        .ok_or_else(|| "Evm contract deployment failed.".into())
    }
}

/// Returns a `StarknetConfig` instance customized for Kakarot.
pub fn kakarot_starknet_config() -> StarknetConfig {
    let kakarot_steps = 2u32.pow(24);
    StarknetConfig {
        allow_zero_max_fee: true,
        auto_mine: true,
        env: Environment {
            chain_id: "SN_GOERLI".into(),
            invoke_max_steps: kakarot_steps,
            validate_max_steps: kakarot_steps,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub struct KakarotTestEnvironment {
    sequencer: TestSequencer,
    kakarot_client: KakarotClient<JsonRpcClient<HttpTransport>>,
    kakarot: DeployedKakarot,
    kakarot_contract: KakarotContract<JsonRpcClient<HttpTransport>>,
    evm_contracts: HashMap<String, Contract>,
}

pub struct Contract {
    pub addresses: ContractAddresses,
    pub abi: Abi,
}

pub struct ContractDeploymentArgs<T: Tokenize> {
    pub name: String,
    pub constructor_args: T,
}

impl KakarotTestEnvironment {
    pub async fn new() -> KakarotTestEnvironment {
        // Construct a Starknet test sequencer
        let sequencer = construct_kakarot_test_sequencer().await;

        // Define the expected funded amount for the Kakarot system
        let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000").unwrap();

        // Deploy the Kakarot system
        let kakarot = deploy_kakarot_system(&sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

        // Create a Kakarot client
        let kakarot_client = KakarotClient::new(
            StarknetClientConfig::new(
                Network::JsonRpcProvider(sequencer.url()),
                kakarot.kakarot_address,
                kakarot.proxy_class_hash,
            ),
            JsonRpcClient::new(HttpTransport::new(sequencer.url())),
        );

        let kakarot_contract =
            KakarotContract::new(kakarot_client.starknet_provider(), kakarot.kakarot_address, kakarot.proxy_class_hash);

        KakarotTestEnvironment { sequencer, kakarot_client, kakarot, kakarot_contract, evm_contracts: HashMap::new() }
    }

    pub async fn deploy_evm_contract<T: Tokenize>(mut self, contract_args: ContractDeploymentArgs<T>) -> Self {
        let kakarot = &self.kakarot;
        let sequencer = &self.sequencer;
        let evm_contracts = &mut self.evm_contracts;

        match kakarot.deploy_evm_contract(sequencer.url(), &contract_args.name, contract_args.constructor_args).await {
            Ok((abi, addresses)) => evm_contracts.insert(contract_args.name, Contract { addresses, abi }),
            Err(err) => panic!("Failed to deploy contract {}: {:?}", contract_args.name, err.to_string()),
        };
        self
    }

    pub fn sequencer(&self) -> &TestSequencer {
        &self.sequencer
    }

    pub fn client(&self) -> &KakarotClient<JsonRpcClient<HttpTransport>> {
        &self.kakarot_client
    }

    pub fn kakarot(&self) -> &DeployedKakarot {
        &self.kakarot
    }

    pub fn evm_contract(&self, name: &str) -> &Contract {
        self.evm_contracts.get(name).unwrap_or_else(|| panic!("could not find contract with name: {}", name))
    }

    pub fn kakarot_contract(&self) -> &KakarotContract<JsonRpcClient<HttpTransport>> {
        &self.kakarot_contract
    }
}

/// Constructs a test sequencer with the Starknet configuration tailored for Kakarot.
///
/// This function initializes a `TestSequencer` instance with the default `SequencerConfig`
/// and a custom `StarknetConfig` obtained from the `get_kakarot_starknet_config()` function.
/// The custom `StarknetConfig` sets the chain_id to "SN_GOERLI" and both the `invoke_max_steps`
/// and `validate_max_steps` to `2**24`. It also sets `allow_zero_max_fee` to true in
/// `StarknetConfig`. This setup is aimed to provide an appropriate environment for testing
/// Kakarot based applications.
///
/// Returns a `TestSequencer` configured for Kakarot.
pub async fn construct_kakarot_test_sequencer() -> TestSequencer {
    TestSequencer::start(SequencerConfig::default(), kakarot_starknet_config()).await
}

/// Asynchronously deploys a Kakarot system to the Starknet network and returns the
/// `DeployedKakarot` object.
///
/// This function deploys a Kakarot system to the network, which includes declaring Kakarot
/// contracts, deploying Kakarot contracts, and deploying and funding an EOA.
pub async fn deploy_kakarot_system(
    starknet_sequencer: &TestSequencer,
    eoa_wallet: EthersLocalWallet,
    funding_amount: FieldElement,
) -> DeployedKakarot {
    dotenv().ok();

    let starknet_account = starknet_sequencer.account();
    let class_hash = declare_kakarot_contracts(&starknet_account).await;
    let eoa_eth_address: Address = eoa_wallet.address().into();
    let eoa_sn_address = {
        let address: Felt252Wrapper = eoa_eth_address.into();
        address.try_into().unwrap()
    };
    let eoa_private_key = {
        let signing_key_bytes = eoa_wallet.signer().to_bytes(); // Convert to bytes
        H256::from_slice(&signing_key_bytes) // Convert to H256
    };
    let fee_token_address = FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap();
    let deployments = deploy_kakarot_contracts(&starknet_account, &class_hash, fee_token_address).await;
    let kkrt_address = deployments.get("kakarot").unwrap();
    let deployed_eoa_sn_address =
        deploy_and_fund_eoa(&starknet_account, *kkrt_address, funding_amount, eoa_sn_address, fee_token_address).await;

    let eoa_addresses = ContractAddresses { eth_address: eoa_eth_address, starknet_address: deployed_eoa_sn_address };

    DeployedKakarot {
        eoa_private_key,
        eoa_addresses,
        kakarot_address: *kkrt_address,
        proxy_class_hash: *class_hash.get("proxy").unwrap(),
        contract_account_class_hash: *class_hash.get("contract_account").unwrap(),
    }
}
