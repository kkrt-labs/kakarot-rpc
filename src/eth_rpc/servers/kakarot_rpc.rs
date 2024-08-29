use crate::{
    config::KakarotRpcConfig,
    eth_rpc::api::kakarot_api::KakarotApiServer,
    pool::{get_retry_tx_interval, get_transaction_max_retries},
    providers::eth_provider::{
        constant::{Constant, MAX_LOGS},
        error::{EthApiError, EthereumDataFormatError, SignatureError},
        provider::EthereumProvider,
        starknet::kakarot_core::{
            get_white_listed_eip_155_transaction_hashes, to_starknet_transaction, MAX_FELTS_IN_CALLDATA,
        },
    },
};
use jsonrpsee::core::{async_trait, RpcResult};
use reth_primitives::B256;
use starknet::{
    core::{
        crypto::compute_hash_on_elements,
        types::{BroadcastedInvokeTransaction, BroadcastedInvokeTransactionV1, Felt},
    },
    providers::Provider,
};
use std::convert::TryInto;
use tracing::instrument;

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
    #[instrument(skip(self))]
    async fn get_starknet_transaction_hash(&self, hash: B256, retries: u8) -> RpcResult<Option<B256>> {
        // Retrieve the stored transaction from the database.
        let transaction = self.eth_provider.transaction_by_hash(hash).await?;

        if let Some(transaction) = transaction {
            // Convert the `Transaction` instance to a `TransactionSigned` instance.
            let transaction_signed_ec_recovered: reth_primitives::TransactionSignedEcRecovered = transaction
                .try_into()
                .map_err(|_| EthApiError::from(EthereumDataFormatError::TransactionConversion))?;

            let (transaction_signed, _) = transaction_signed_ec_recovered.to_components();

            // Retrieve the signer of the transaction.
            let signer =
                transaction_signed.recover_signer().ok_or_else(|| EthApiError::from(SignatureError::Recovery))?;
            // Create the Starknet transaction.
            let starknet_transaction = to_starknet_transaction(&transaction_signed, signer, retries)
                .map_err(|_| EthApiError::from(EthereumDataFormatError::TransactionConversion))?
                .try_into_v1()
                .map_err(|_| EthApiError::from(EthereumDataFormatError::TransactionConversion))?;

            let chain_id = self.starknet_provider.chain_id().await.unwrap();

            // Compute the hash on elements
            let transaction_hash = compute_hash_on_elements(&[
                Felt::from_bytes_be_slice(b"invoke"),
                Felt::ONE,
                starknet_transaction.sender_address,
                Felt::ZERO,
                compute_hash_on_elements(&starknet_transaction.calldata),
                starknet_transaction.max_fee,
                chain_id,
                starknet_transaction.nonce,
            ]);

            Ok(Some(B256::from_slice(&transaction_hash.to_bytes_be()[..])))
        } else {
            Ok(None)
        }
    }

    async fn get_config(&self) -> RpcResult<Constant> {
        let starknet_config = KakarotRpcConfig::from_env().expect("Failed to load Kakarot RPC config");
        Ok(Constant {
            max_logs: *MAX_LOGS,
            starknet_network: String::from(starknet_config.network_url),
            retry_tx_interval: get_retry_tx_interval(),
            transaction_max_retries: get_transaction_max_retries(),
            max_felts_in_calldata: *MAX_FELTS_IN_CALLDATA,
            white_listed_eip_155_transaction_hashes: get_white_listed_eip_155_transaction_hashes(),
        })
    }
}
