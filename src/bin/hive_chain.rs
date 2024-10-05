#![allow(clippy::significant_drop_tightening)]

use alloy_rlp::Decodable;
use clap::Parser;
use kakarot_rpc::{
    into_via_try_wrapper,
    providers::{
        eth_provider::{constant::CHAIN_ID, starknet::relayer::LockedRelayer},
        sn_provider::StarknetProvider,
    },
};
use reth_primitives::{bytes::Buf, Block, BlockBody, BytesMut};
use starknet::{
    core::types::{BlockId, BlockTag, Felt},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};
use std::{path::PathBuf, str::FromStr};
use tokio::{fs::File, io::AsyncReadExt, sync::Mutex};
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, FramedRead};
use url::Url;

struct BlockFileCodec;

impl Decoder for BlockFileCodec {
    type Item = Block;
    type Error = eyre::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }
        let buf_slice = &mut src.as_ref();
        let body = Block::decode(buf_slice)?;
        src.advance(src.len() - buf_slice.len());
        Ok(Some(body))
    }
}

/// The inputs to the binary.
#[derive(Parser, Debug)]
pub struct Args {
    /// The path to the chain file for the hive test.
    #[clap(short, long)]
    chain_path: PathBuf,
    /// The relayer address.
    #[clap(long)]
    relayer_address: Felt,
    /// The relayer private key.
    #[clap(long)]
    relayer_pk: Felt,
}

const STARKNET_RPC_URL: &str = "http://0.0.0.0:5050";
const MAX_FELTS_IN_CALLDATA: &str = "30000";

// Define the modulo constant
const CHAIN_ID_MODULO: u64 = 1u64 << 53;

// Utility function to apply the chain ID modulo
fn apply_chain_id_modulo(chain_id: u64) -> u64 {
    chain_id % CHAIN_ID_MODULO
}

/// Inspired by the Import command from Reth.
/// https://github.com/paradigmxyz/reth/blob/main/bin/reth/src/commands/import.rs
#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    // Get the provider
    let provider = JsonRpcClient::new(HttpTransport::new(Url::from_str(STARKNET_RPC_URL)?));
    let starknet_provider = StarknetProvider::new(provider);

    // Set the env
    std::env::set_var("RELAYER_PRIVATE_KEY", format!("0x{:x}", args.relayer_pk));
    std::env::set_var("MAX_FELTS_IN_CALLDATA", MAX_FELTS_IN_CALLDATA);
    std::env::set_var("STARKNET_NETWORK", STARKNET_RPC_URL);

    // Set the chain id
    let chain_id = starknet_provider.chain_id().await?;
    let chain_id_mod = apply_chain_id_modulo(chain_id);

    // Initialize the chain ID globally
    let _ = CHAIN_ID.get_or_init(|| Felt::from(chain_id_mod));

    // Prepare the relayer
    let relayer_balance = starknet_provider.balance_at(args.relayer_address, BlockId::Tag(BlockTag::Latest)).await?;
    let relayer_balance = into_via_try_wrapper!(relayer_balance)?;

    let current_nonce = Mutex::new(Felt::ZERO);
    let mut relayer = LockedRelayer::new(
        current_nonce.lock().await,
        args.relayer_address,
        relayer_balance,
        JsonRpcClient::new(HttpTransport::new(Url::from_str(STARKNET_RPC_URL)?)),
        chain_id,
    );

    // Read the rlp file
    let mut file = File::open(args.chain_path).await?;

    let metadata = file.metadata().await?;
    let file_len = metadata.len();

    // Read the entire file into memory
    let mut reader = vec![];
    file.read_to_end(&mut reader).await?;
    let mut stream = FramedRead::with_capacity(&reader[..], BlockFileCodec, file_len as usize);

    // Extract the block
    let mut bodies: Vec<BlockBody> = Vec::new();
    while let Some(block_res) = stream.next().await {
        let block = block_res?;
        bodies.push(block.into());
    }

    for (block_number, body) in bodies.into_iter().enumerate() {
        while starknet_provider.block_number().await? < block_number as u64 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        for transaction in &body.transactions {
            relayer.relay_transaction(transaction).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Increase the relayer's nonce
            let nonce = relayer.nonce_mut();
            *nonce += Felt::ONE;
        }
    }

    Ok(())
}