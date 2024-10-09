use alloy_primitives::{B256, U256};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::LazyLock};

/// Maximum priority fee per gas
pub static MAX_PRIORITY_FEE_PER_GAS: LazyLock<u64> = LazyLock::new(|| 0);

/// Maximum number of logs that can be fetched in a single request
pub static MAX_LOGS: LazyLock<Option<u64>> =
    LazyLock::new(|| std::env::var("MAX_LOGS").ok().and_then(|val| u64::from_str(&val).ok()));

/// Gas limit for estimate gas and call
pub const CALL_REQUEST_GAS_LIMIT: u64 = 50_000_000;
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

/// Struct used to return the constant values from the `kakarot_getConfig` endpoint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Constant {
    /// Maximum number of logs to output for `eth_getLogs` RPC Method
    pub max_logs: Option<u64>,
    /// Name of the `StarkNet` network.
    pub starknet_network: String,
    /// Maximum number of Felts in calldata.
    pub max_felts_in_calldata: usize,
    /// List of whitelisted hashes allow to submit pre EIP-155 transactions.
    pub white_listed_eip_155_transaction_hashes: Vec<B256>,
}

#[cfg(feature = "hive")]
pub mod hive {
    use std::{
        env::var,
        str::FromStr,
        sync::{Arc, LazyLock},
    };

    use starknet::{
        accounts::{ConnectedAccount, ExecutionEncoding, SingleOwnerAccount},
        core::types::Felt,
        providers::{jsonrpc::HttpTransport, JsonRpcClient},
        signers::{LocalWallet, SigningKey},
    };
    use tokio::sync::Mutex;

    use crate::constants::{KAKAROT_RPC_CONFIG, STARKNET_CHAIN_ID};

    pub static DEPLOY_WALLET: LazyLock<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>> =
        LazyLock::new(|| {
            SingleOwnerAccount::new(
                JsonRpcClient::new(HttpTransport::new(KAKAROT_RPC_CONFIG.network_url.clone())),
                LocalWallet::from_signing_key(SigningKey::from_secret_scalar(
                    Felt::from_str(&var("KATANA_PRIVATE_KEY").expect("Missing deployer private key"))
                        .expect("Failed to parse deployer private key"),
                )),
                Felt::from_str(&var("KATANA_ACCOUNT_ADDRESS").expect("Missing deployer address"))
                    .expect("Failed to parse deployer address"),
                *STARKNET_CHAIN_ID,
                ExecutionEncoding::New,
            )
        });
    pub static DEPLOY_WALLET_NONCE: LazyLock<Arc<Mutex<Felt>>> = LazyLock::new(|| {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                Arc::new(Mutex::new(DEPLOY_WALLET.get_nonce().await.expect("failed to fetch deploy wallet nonce")))
            })
        })
    });
}
