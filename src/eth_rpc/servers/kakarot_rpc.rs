use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::kakarot_api::KakarotApiServer;
use jsonrpsee::core::{async_trait, RpcResult};
use starknet::core::types::FieldElement;
use starknet::{
    types::{Transaction, TransactionSigned, TransactionSignedEcRecovered},
    utils::{compute_hash_on_elements, to_starknet_transaction},
    Provider,
};
use std::convert::TryInto;
// use reth_primitives::TransactionSignedEcRecovered;
// use reth_primitives::{
//     Address, BlockId, BlockNumberOrTag, Bytes, TransactionSigned, TransactionSignedEcRecovered, TxKind, B256, U256, U64,
// };
use reth_primitives::B256;

#[derive(Debug)]
pub struct KakarotRpc<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> KakarotRpc<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> KakarotApiServer for KakarotRpc<P> {
    async fn kakarot_get_starknet_transaction_hash(&self, hash: B256, retries: u8) -> RpcResult<FieldElement> {
        // Retrieve the stored transaction from the database.
        let transaction = self.eth_provider.transaction_by_hash(hash).await?;

        // Convert the `Transaction` instance to a `TransactionSigned` instance.
        let transaction_signed_ec_recovered: TransactionSignedEcRecovered = transaction.try_into()?;
        let transaction_signed = transaction_signed_ec_recovered.to_components();

        // Retrieve the signer of the transaction.
        let signer = transaction_signed.recover_signer()?;

        // Create the Starknet transaction.
        let starknet_transaction = to_starknet_transaction(transaction_signed, signer, 0, retries)?;

        // Compute the hash of the transaction.
        let hash = compute_hash_on_elements(&starknet_transaction.to_elements())?;

        // Return the computed hash.
        Ok(hash)
    }
}
