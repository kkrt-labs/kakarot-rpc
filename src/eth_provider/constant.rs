use lazy_static::lazy_static;

lazy_static! {
    pub static ref MAX_PRIORITY_FEE_PER_GAS: u64 = 0;
}

#[cfg(feature = "hive")]
use {
    crate::config::KakarotRpcConfig,
    starknet::{
        accounts::{ExecutionEncoding, SingleOwnerAccount},
        providers::{jsonrpc::HttpTransport, JsonRpcClient},
        signers::{LocalWallet, SigningKey},
    },
    starknet_crypto::FieldElement,
    std::{env::var, str::FromStr, sync::OnceLock},
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
            *CHAIN_ID.get().expect("Missing chain ID"),
            ExecutionEncoding::New
        );
}
