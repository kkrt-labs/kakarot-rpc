use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::kakarot_api::KakarotApiServer;
use jsonrpsee::core::{async_trait, RpcResult};
use jsonrpsee::core::{async_trait, RpcResult};
use reth_primitives::B256;
use starknet::core::types::FieldElement;
use starknet::{
    types::{Transaction, TransactionSigned, TransactionSignedEcRecovered},
    utils::{compute_hash_on_elements, to_starknet_transaction},
    Provider,
};
use std::convert::TryInto;

#[derive(Debug)]
pub struct KakarotRpc<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> KakarotRpc<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}
trait ToElements {
    fn try_into_v1(self) -> Result<BroadcastedInvokeTransactionV1, &'static str>;
}

impl ToElements for BroadcastedInvokeTransaction {
    fn try_into_v1(self) -> Result<BroadcastedInvokeTransactionV1, &'static str> {
        match self {
            BroadcastedInvokeTransaction::V1(tx_v1) => Ok(tx_v1),
            BroadcastedInvokeTransaction::V3(_) => Err("Transaction is V3, cannot convert to V1"),
        }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> KakarotApiServer for KakarotRpc<P> {
    async fn kakarot_get_starknet_transaction_hash(&self, hash: B256, retries: u8) -> RpcResult<FieldElement> {
        // Retrieve the stored transaction from the database.
        let transaction: reth_rpc_types::Transaction =
            self.eth_provider.transaction_by_hash(hash).await.unwrap().unwrap();

        // Convert the `Transaction` instance to a `TransactionSigned` instance.
        let transaction_signed_ec_recovered: reth_primitives::TransactionSignedEcRecovered =
            <reth_rpc_types::Transaction as TryInto<reth_primitives::TransactionSignedEcRecovered>>::try_into(
                transaction,
            )
            .unwrap();
        let (transaction_signed, _address) = transaction_signed_ec_recovered.to_components();

        // Retrieve the signer of the transaction.
        let signer = transaction_signed.recover_signer().unwrap();
        // Create the Starknet transaction.
        let starknet_transaction =
            (to_starknet_transaction(&transaction_signed, signer, retries).unwrap()).try_into_v1().unwrap();

        // invoke prefix
        const PREFIX_INVOKE: FieldElement = FieldElement::from_mont([
            18443034532770911073,
            18446744073709551615,
            18446744073709551615,
            513398556346534256,
        ]);

        let chain_id = FieldElement::from(transaction_signed.chain_id().expect("Chain ID is None"));

        // Compute the hash on elements
        let transaction_hash = compute_hash_on_elements(&[
            PREFIX_INVOKE,
            FieldElement::ONE,
            starknet_transaction.sender_address,
            FieldElement::ZERO,
            starknet_transaction.max_fee,
            compute_hash_on_elements(&starknet_transaction.calldata),
            chain_id,
            starknet_transaction.nonce,
        ]);

        Ok(transaction_hash)
    }
}
