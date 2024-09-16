#![allow(clippy::significant_drop_tightening)]

use alloy_rlp::Decodable;
use kakarot_rpc::{
    into_via_try_wrapper,
    providers::{eth_provider::starknet::relayer::LockedRelayer, sn_provider::StarknetProvider},
};
use reth_primitives::{bytes::Buf, Block, BlockBody, BytesMut};
use starknet::{
    core::types::{BlockId, BlockTag, Felt},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};
use std::{path::Path, str::FromStr};
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

/// Inspired by the Import command from Reth.
/// https://github.com/paradigmxyz/reth/blob/main/bin/reth/src/commands/import.rs
#[tokio::main]
async fn main() -> eyre::Result<()> {
    let chain_path = Path::new(&std::env::var("CHAIN_PATH")?).to_path_buf();
    let relayer_address = Felt::from_str(&std::env::var("RELAYER_ADDRESS")?)?;

    let mut file = File::open(chain_path).await?;

    let metadata = file.metadata().await?;
    let file_len = metadata.len();

    // read the entire file into memory
    let mut reader = vec![];
    file.read_to_end(&mut reader).await.unwrap();
    let mut stream = FramedRead::with_capacity(&reader[..], BlockFileCodec, file_len as usize);

    let mut bodies: Vec<BlockBody> = Vec::new();
    while let Some(block_res) = stream.next().await {
        let block = block_res?;
        bodies.push(block.into());
    }

    let provider = JsonRpcClient::new(HttpTransport::new(Url::from_str(&std::env::var("STARKNET_NETWORK")?)?));
    let starknet_provider = StarknetProvider::new(provider);
    let relayer_balance = starknet_provider.balance_at(relayer_address, BlockId::Tag(BlockTag::Latest)).await?;
    let relayer_balance = into_via_try_wrapper!(relayer_balance)?;

    let current_nonce = Mutex::new(Felt::ZERO);
    let mut relayer = LockedRelayer::new(current_nonce.lock().await, relayer_address, relayer_balance);

    for (block_number, body) in bodies.into_iter().enumerate() {
        while starknet_provider.block_number().await? < block_number as u64 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        for transaction in &body.transactions {
            relayer.relay_transaction(transaction).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let nonce = relayer.nonce_mut();
            *nonce += Felt::ONE;
        }
    }

    Ok(())
}
