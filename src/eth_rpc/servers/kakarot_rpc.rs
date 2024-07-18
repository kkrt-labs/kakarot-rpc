use crate::{
    eth_provider::{provider::EthereumProvider, starknet::kakarot_core::to_starknet_transaction},
    eth_rpc::api::kakarot_api::KakarotApiServer,
};
use jsonrpsee::core::{async_trait, RpcResult};
use reth_primitives::B256;
use starknet::{
    core::{
        crypto::compute_hash_on_elements,
        types::{BroadcastedInvokeTransaction, BroadcastedInvokeTransactionV1},
    },
    providers::Provider,
};
use std::convert::TryInto;
use starknet_crypto::FieldElement;
use jsonrpsee_types::ErrorObject;
use jsonrpsee_types::error::INVALID_PARAMS_CODE;

#[derive(Debug)]
pub struct KakarotRpc<EP, SP> {
    eth_provider: EP,
    starknet_provider: SP,
}

impl<EP, SP> KakarotRpc<EP, SP> {
    pub const fn new(eth_provider: EP, starknet_provider: SP) -> Self {
        Self { eth_provider, starknet_provider }
    }
}
trait ToElements {
    fn try_into_v1(self) -> Result<BroadcastedInvokeTransactionV1, eyre::Error>;
}

impl ToElements for BroadcastedInvokeTransaction {
    fn try_into_v1(self) -> Result<BroadcastedInvokeTransactionV1, eyre::Error> {
        match self {
            Self::V1(tx_v1) => Ok(tx_v1),
            Self::V3(_) => Err(eyre::eyre!("Transaction is V3, cannot convert to V1")),
        }
    }
}

#[async_trait]
impl<EP, SP> KakarotApiServer for KakarotRpc<EP, SP>
where
    EP: EthereumProvider + Send + Sync + 'static,
    SP: Provider + Send + Sync + 'static,
{
    async fn get_starknet_transaction_hash(&self, hash: B256, retries: u8) -> RpcResult<B256> {
        // Retrieve the stored transaction from the database.
        let transaction = self.eth_provider.transaction_by_hash(hash).await
            .map_err(ErrorObject::from)?
            .ok_or_else(|| ErrorObject::owned(
                INVALID_PARAMS_CODE,
                "Transaction not found",
                None::<()>
            ))?;

        // Convert the `Transaction` instance to a `TransactionSigned` instance.
        let transaction_signed_ec_recovered: reth_primitives::TransactionSignedEcRecovered = transaction
            .try_into()
            .map_err(|_| ErrorObject::owned(
                INVALID_PARAMS_CODE,
                "Failed to convert transaction",
                None::<()>
            ))?;

        let (transaction_signed, _) = transaction_signed_ec_recovered.to_components();

        // Retrieve the signer of the transaction.
        let signer = transaction_signed
            .recover_signer()
            .ok_or_else(|| ErrorObject::owned(
                INVALID_PARAMS_CODE,
                "Failed to recover signer",
                None::<()>
            ))?;
        // Create the Starknet transaction.
        let starknet_transaction = to_starknet_transaction(&transaction_signed, signer, retries)
            .map_err(|_| ErrorObject::owned(
                INVALID_PARAMS_CODE,
                "Failed to convert to StarkNet transaction",
                None::<()>
            ))?
            .try_into_v1()
            .map_err(|_| ErrorObject::owned(
                INVALID_PARAMS_CODE,
                "Failed to convert StarkNet transaction to version 1",
                None::<()>
            ))?;

        let chain_id = self.starknet_provider.chain_id().await.unwrap();

        // Compute the hash on elements
        let transaction_hash = compute_hash_on_elements(&[
            FieldElement::from_byte_slice_be(b"invoke").unwrap(),
            FieldElement::ONE,
            starknet_transaction.sender_address,
            FieldElement::ZERO,
            compute_hash_on_elements(&starknet_transaction.calldata),
            starknet_transaction.max_fee,
            chain_id,
            starknet_transaction.nonce,
        ]);

        Ok(B256::from_slice(&transaction_hash.to_bytes_be()[..]))
    }
}
