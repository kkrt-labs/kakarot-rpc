pub const ACCOUNT_EVM_ADDRESS: &str = "Account_evm_address";
pub const ACCOUNT_IMPLEMENTATION: &str = "Account_implementation";
pub const ACCOUNT_NONCE: &str = "Account_nonce";
pub const ACCOUNT_STORAGE: &str = "Account_storage";
pub const OWNABLE_OWNER: &str = "Ownable_owner";
pub const ACCOUNT_CAIRO1_HELPERS_CLASS_HASH: &str = "Account_cairo1_helpers_class_hash";
pub const ACCOUNT_AUTHORIZED_MESSAGE_HASHES: &str = "Account_authorized_message_hashes";
/// Pre EIP 155 authorized message hashes. Presently contains:
///   - The Arachnid deployer message hash.
pub const EIP_155_AUTHORIZED_MESSAGE_HASHES: [&str; 1] =
    ["0x3de642d76cf5cf9ffcf9b51e11b3b21e09f63278ed94a89281ca8054b2225434"];

pub const KAKAROT_EVM_TO_STARKNET_ADDRESS: &str = "Kakarot_evm_to_starknet_address";
pub const KAKAROT_NATIVE_TOKEN_ADDRESS: &str = "Kakarot_native_token_address";
pub const KAKAROT_ACCOUNT_CONTRACT_CLASS_HASH: &str = "Kakarot_account_contract_class_hash";
pub const KAKAROT_UNINITIALIZED_ACCOUNT_CLASS_HASH: &str = "Kakarot_uninitialized_account_class_hash";
pub const KAKAROT_CAIRO1_HELPERS_CLASS_HASH: &str = "Kakarot_cairo1_helpers_class_hash";
pub const KAKAROT_COINBASE: &str = "Kakarot_coinbase";
pub const KAKAROT_BASE_FEE: &str = "Kakarot_base_fee";
pub const KAKAROT_PREV_RANDAO: &str = "Kakarot_prev_randao";
pub const KAKAROT_BLOCK_GAS_LIMIT: &str = "Kakarot_block_gas_limit";
pub const KAKAROT_CHAIN_ID: &str = "Kakarot_chain_id";
