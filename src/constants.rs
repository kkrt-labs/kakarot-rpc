use crate::{config::KakarotRpcConfig, eth_rpc::config::RPCConfig};
use num_traits::ToPrimitive;
use starknet::{
    core::types::{Felt, NonZeroFelt},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};
use std::sync::LazyLock;

/// The max chain id allowed by [Metamask](https://gist.github.com/rekmarks/a47bd5f2525936c4b8eee31a16345553)
pub static MAX_CHAIN_ID: u64 = (2u64.pow(53) - 39) / 2;
/// The chain id of the underlying Starknet chain.
pub static STARKNET_CHAIN_ID: LazyLock<Felt> = LazyLock::new(|| {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            let provider = JsonRpcClient::new(HttpTransport::new(KAKAROT_RPC_CONFIG.network_url.clone()));
            provider.chain_id().await.expect("failed to get chain for chain")
        })
    })
});
/// The chain id for the Ethereum chain running on the Starknet chain.
pub static ETH_CHAIN_ID: LazyLock<u64> = LazyLock::new(|| {
    STARKNET_CHAIN_ID.div_rem(&NonZeroFelt::from_felt_unchecked(Felt::from(MAX_CHAIN_ID))).1.to_u64().expect("modulo")
});

/// The Kakarot RPC configuration.
pub static KAKAROT_RPC_CONFIG: LazyLock<KakarotRpcConfig> =
    LazyLock::new(|| KakarotRpcConfig::from_env().expect("failed to load Kakarot RPC config"));

/// The RPC configuration.
pub static RPC_CONFIG: LazyLock<RPCConfig> =
    LazyLock::new(|| RPCConfig::from_env().expect("failed to load RPC config"));

/// The gas limit for Kakarot blocks.
pub const KAKAROT_BLOCK_GAS_LIMIT: u64 = 7_000_000;
