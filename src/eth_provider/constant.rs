use lazy_static::lazy_static;
use reth_primitives::U256;

lazy_static! {
    pub static ref MAX_PRIORITY_FEE_PER_GAS: u64 = 0;
}

/// Gas limit for estimate gas and call
pub const CALL_REQUEST_GAS_LIMIT: u128 = 5_000_000;
/// Number of characters for representing a U256 in a hex string form. Used for padding hashes
pub const HASH_HEX_STRING_LEN: usize = 64;
/// Number of characters for representing logs topics in a hex string form. Used for padding logs topics
pub const LOGS_TOPICS_HEX_STRING_LEN: usize = HASH_HEX_STRING_LEN;
/// Number of characters for representing a u64 in a hex string form. Used for padding numbers
pub const U64_HEX_STRING_LEN: usize = 16;
/// Number of characters for representing a block number in a hex string form. Used for padding block numbers
pub const BLOCK_NUMBER_HEX_STRING_LEN: usize = U64_HEX_STRING_LEN;
/// Number of characters for representing an address in a hex string form. Used for padding addresses
pub const ADDRESS_HEX_STRING_LEN: usize = 40;
/// Starknet Modulus: 0x800000000000011000000000000000000000000000000000000000000000001
pub const STARKNET_MODULUS: U256 = U256::from_limbs([0x1, 0, 0, 0x0800_0000_0000_0011]);
/// Maximum number of times a transaction can be retried
pub const TRANSACTION_MAX_RETRIES: u64 = 10;

#[cfg(feature = "hive")]
use {
    crate::config::KakarotRpcConfig,
    starknet::{
        accounts::{ExecutionEncoding, SingleOwnerAccount},
        providers::{jsonrpc::HttpTransport, JsonRpcClient},
        signers::{LocalWallet, SigningKey},
    },
    starknet_crypto::FieldElement,
    std::sync::Arc,
    std::{env::var, str::FromStr, sync::OnceLock},
    tokio::sync::Mutex,
};

#[cfg(feature = "hive")]
pub static CHAIN_ID: OnceLock<FieldElement> = OnceLock::new();

#[cfg(feature = "hive")]
lazy_static! {
    static ref RPC_CONFIG: KakarotRpcConfig = KakarotRpcConfig::from_env().expect("Failed to load Kakarot RPC config");
    pub static ref DEPLOY_WALLET: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet> =
        SingleOwnerAccount::new(
            JsonRpcClient::new(HttpTransport::new(RPC_CONFIG.network.provider_url().expect("Incorrect provider URL"))),
            LocalWallet::from_signing_key(SigningKey::from_secret_scalar(
                FieldElement::from_str(&var("KATANA_PRIVATE_KEY").expect("Missing deployer private key"))
                    .expect("Failed to parse deployer private key")
            )),
            FieldElement::from_str(&var("KATANA_ACCOUNT_ADDRESS").expect("Missing deployer address"))
                .expect("Failed to parse deployer address"),
            *CHAIN_ID.get().expect("Failed to get chain id"),
            ExecutionEncoding::New
        );
    pub static ref DEPLOY_WALLET_NONCE: Arc<Mutex<FieldElement>> = Arc::new(Mutex::new(FieldElement::ZERO));
}
