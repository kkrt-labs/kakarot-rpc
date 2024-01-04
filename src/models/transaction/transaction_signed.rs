use reth_primitives::{Bytes, TransactionSigned};
use reth_rlp::Decodable as _;
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransaction};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::starknet_client::constants::{CHAIN_ID, MAX_FEE};
use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::helpers::{
    prepare_kakarot_eth_send_transaction, split_u256_into_field_elements, DataDecodingError,
};
use crate::starknet_client::KakarotClient;

use reth_primitives::Transaction as TransactionType;

pub struct StarknetTransactionSigned(Bytes);

impl From<Bytes> for StarknetTransactionSigned {
    fn from(tx: Bytes) -> Self {
        Self(tx)
    }
}

impl StarknetTransactionSigned {
    pub async fn to_broadcasted_invoke_transaction<P: Provider + Send + Sync>(
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

        // Get estimated_fee from Starknet
        let max_fee = *MAX_FEE;

        // Step: Signature
        // Extract the signature from the Ethereum Transaction
        // and place it in the Starknet signature InvokeTransaction vector
        let mut signature: Vec<FieldElement> = {
            let r = split_u256_into_field_elements(transaction.signature().r);
            let s = split_u256_into_field_elements(transaction.signature().s);
            let signature = vec![r[0], r[1], s[0], s[1]];
            signature
        };
        // Push the last element of the signature
        // In case of a Legacy Transaction, it is v := {0, 1} + chain_id * 2 + 35
        // Else, it is odd_y_parity
        if let TransactionType::Legacy(_) = transaction.transaction {
            let chain_id = CHAIN_ID;
            // TODO(elias): replace by dynamic chain_id when Kakarot supports it
            // let chain_id: u64 = client
            //     .starknet_provider()
            //     .chain_id()
            //     .await?
            //     .try_into()
            //     .map_err(|e: ValueOutOfRangeError| ConversionError::ValueOutOfRange(e.to_string()))?;
            signature.push(transaction.signature().v(Some(chain_id)).into());
        } else {
            signature.push((transaction.signature().odd_y_parity as u64).into());
        }

        // Step: Calldata
        // RLP encode the transaction without the signature
        // Example: For Legacy Transactions: rlp([nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0])
        let mut signed_data = Vec::new();
        transaction.transaction.encode_without_signature(&mut signed_data);

        let calldata = prepare_kakarot_eth_send_transaction(
            client.kakarot_address(),
            signed_data.into_iter().map(FieldElement::from).collect(),
        );

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
