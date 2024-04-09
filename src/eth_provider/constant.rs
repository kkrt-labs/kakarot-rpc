use lazy_static::lazy_static;

lazy_static! {
    /// Maximum priority fee per gas.
    pub static ref MAX_PRIORITY_FEE_PER_GAS: u64 = 0;
}

/// Gas limit for call requests.
pub const CALL_REQUEST_GAS_LIMIT: u64 = 5_000_000;

/// Padding size for hash values.
pub const HASH_PADDING: usize = 64;

/// Padding size for u64 values.
pub const U64_PADDING: usize = 16;

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

/// OnceLock instance for storing the chain ID.
#[cfg(feature = "hive")]
pub static CHAIN_ID: OnceLock<FieldElement> = OnceLock::new();

#[cfg(feature = "hive")]
lazy_static! {
    /// Configuration for Kakarot RPC.
    static ref RPC_CONFIG: KakarotRpcConfig = KakarotRpcConfig::from_env().expect("Failed to load Kakarot RPC config");

    /// Wallet used for deployment.
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

    /// Wallet used for deployment.
    pub static ref DEPLOY_WALLET_NONCE: Arc<Mutex<FieldElement>> = Arc::new(Mutex::new(FieldElement::ZERO));
}
