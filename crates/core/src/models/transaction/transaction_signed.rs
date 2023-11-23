use async_trait::async_trait;
use reth_primitives::{Bytes, TransactionSigned};
use reth_rlp::Decodable as _;
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransaction};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::constants::MAX_FEE;
use crate::client::errors::EthApiError;
use crate::client::helpers::{raw_kakarot_calldata, DataDecodingError};
use crate::client::KakarotClient;
use crate::models::convertible::ConvertibleSignedTransaction;

pub struct StarknetTransactionSigned(Bytes);

impl From<Bytes> for StarknetTransactionSigned {
    fn from(tx: Bytes) -> Self {
        Self(tx)
    }
}

#[async_trait]
impl ConvertibleSignedTransaction for StarknetTransactionSigned {
    async fn to_broadcasted_invoke_transaction<P: Provider + Send + Sync + 'static>(
        &self,
        client: &KakarotClient<P>,
    ) -> Result<BroadcastedInvokeTransaction, EthApiError> {
        let mut data = self.0.as_ref();

        let transaction = TransactionSigned::decode(&mut data).map_err(DataDecodingError::TransactionDecodingError)?;

        let evm_address = transaction.recover_signer().ok_or_else(|| {
            EthApiError::Other(anyhow::anyhow!("Kakarot send_transaction: signature ecrecover failed"))
        })?;

        let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);

        let starknet_address = client.compute_starknet_address(&evm_address, &starknet_block_id).await?;

        let nonce = FieldElement::from(transaction.nonce());

        let calldata = raw_kakarot_calldata(
            client.kakarot_address(),
            self.0.to_vec().into_iter().map(FieldElement::from).collect(),
        );

        // Get estimated_fee from Starknet
        let max_fee = *MAX_FEE;

        let signature = vec![];

        Ok(BroadcastedInvokeTransaction {
            max_fee,
            signature,
            nonce,
            sender_address: starknet_address,
            calldata,
            is_query: false,
        })
    }
}
