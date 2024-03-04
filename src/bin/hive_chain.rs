use alloy_rlp::Decodable;
use eyre::eyre;
use kakarot_rpc::eth_provider::starknet::kakarot_core::to_starknet_transaction;
use reth_primitives::{bytes::Buf, Block, BlockBody, BytesMut};
use starknet::{
    core::types::BroadcastedInvokeTransaction,
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
};
use starknet_crypto::FieldElement;
use std::{path::Path, str::FromStr};
use tokio::{fs::File, io::AsyncReadExt};
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
    let chain_path = Path::new(&std::env::var("CHAIN_PATH").expect("Failed to load CHAIN_PATH var")).to_path_buf();
    let mut file = File::open(chain_path).await?;

    let metadata = file.metadata().await?;
    let file_len = metadata.len();

    // read the entire file into memory
    let mut reader = vec![];
    file.read_to_end(&mut reader).await.unwrap();
    let mut stream = FramedRead::with_capacity(&reader[..], BlockFileCodec, file_len as usize);

    let mut bodies = Vec::new();
    while let Some(block_res) = stream.next().await {
        let block = block_res?;

        bodies.push(BlockBody { transactions: block.body, ommers: block.ommers, withdrawals: block.withdrawals });
    }

    let provider = JsonRpcClient::new(HttpTransport::new(Url::from_str("http://localhost:5050")?));
    let mut current_nonce = FieldElement::ZERO;

    for (block_number, body) in bodies.into_iter().enumerate() {
        while provider.block_number().await? < block_number as u64 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        for transaction in body.transactions {
            let signer = transaction.recover_signer().ok_or(eyre!("Failed to recover signer"))?;
            let chain_id = transaction.chain_id().ok_or(eyre!("Failed to recover chain id"))?;
            let starknet_tx = to_starknet_transaction(&transaction, chain_id, signer)?;

            // Stop if the nonce is incorrect
            assert_eq!(starknet_tx.nonce, current_nonce);

            provider.add_invoke_transaction(BroadcastedInvokeTransaction::V1(starknet_tx)).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            current_nonce += 1u8.into();
        }
    }

    Ok(())
}
