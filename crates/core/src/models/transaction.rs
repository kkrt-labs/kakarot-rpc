use async_trait::async_trait;
use reth_primitives::{H256, U256};
use reth_rpc_types::{Signature, Transaction as EthTransaction};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, FieldElement, InvokeTransaction, Transaction};
use starknet::providers::Provider;

use super::felt::Felt252Wrapper;
use super::ConversionError;
use crate::client::client_api::KakarotProvider;
use crate::client::constants::{self, CHAIN_ID};
use crate::client::errors::EthApiError;
use crate::client::helpers::{decode_signature_from_tx_calldata, vec_felt_to_bytes};
use crate::models::convertible::ConvertibleStarknetTransaction;

pub struct StarknetTransaction(Transaction);

impl From<Transaction> for StarknetTransaction {
    fn from(tx: Transaction) -> Self {
        Self(tx)
    }
}

impl From<StarknetTransaction> for Transaction {
    fn from(tx: StarknetTransaction) -> Self {
        tx.0
    }
}

macro_rules! get_invoke_transaction_field {
    (($field_v0:ident, $field_v1:ident), $type:ty) => {
        pub fn $field_v1(&self) -> Result<$type, ConversionError> {
            match &self.0 {
                Transaction::Invoke(tx) => match tx {
                    InvokeTransaction::V0(tx) => Ok(tx.$field_v0.clone().into()),
                    InvokeTransaction::V1(tx) => Ok(tx.$field_v1.clone().into()),
                },
                _ => Err(ConversionError::TransactionConversionError(
                    constants::error_messages::INVALID_TRANSACTION_TYPE.to_string(),
                )),
            }
        }
    };
}

impl StarknetTransaction {
    get_invoke_transaction_field!((transaction_hash, transaction_hash), Felt252Wrapper);
    get_invoke_transaction_field!((nonce, nonce), Felt252Wrapper);
    get_invoke_transaction_field!((calldata, calldata), Vec<FieldElement>);
    get_invoke_transaction_field!((contract_address, sender_address), Felt252Wrapper);
}

pub struct StarknetTransactions(Vec<Transaction>);

impl From<Vec<Transaction>> for StarknetTransactions {
    fn from(txs: Vec<Transaction>) -> Self {
        Self(txs)
    }
}

impl From<StarknetTransactions> for Vec<Transaction> {
    fn from(txs: StarknetTransactions) -> Self {
        txs.0
    }
}

#[async_trait]
impl ConvertibleStarknetTransaction for StarknetTransaction {
    async fn to_eth_transaction(
        &self,
        client: &dyn KakarotProvider,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError> {
        if !self.is_kakarot_tx(client).await? {
            return Err(EthApiError::OtherError(anyhow::anyhow!("Kakarot Filter: Tx is not part of Kakarot")));
        }

        let starknet_block_latest = StarknetBlockId::Tag(BlockTag::Latest);
        let sender_address: FieldElement = self.sender_address()?.into();

        let hash: H256 = self.transaction_hash()?.into();

        let nonce: U256 = self.nonce()?.into();

        let from = client.get_evm_address(&sender_address, &starknet_block_latest).await?;

        let max_priority_fee_per_gas = Some(client.max_priority_fee_per_gas());

        let calldata = self.calldata().unwrap_or_default();
        let input = vec_felt_to_bytes(calldata.clone());

        // TODO: wrap to abstract the following lines?
        // Extracting the signature
        let signature = decode_signature_from_tx_calldata(&calldata)?;
        let v = if signature.odd_y_parity { 1 } else { 0 } + 35 + 2 * CHAIN_ID;
        let signature = Some(Signature { r: signature.r, s: signature.s, v: U256::from_limbs_slice(&[v]) });

        Ok(EthTransaction {
            hash,
            nonce,
            block_hash,
            block_number,
            transaction_index,
            from,
            to: None,               // TODO fetch the to
            value: U256::from(100), // TODO fetch the value
            gas_price: None,        // TODO fetch the gas price
            gas: U256::from(100),   // TODO fetch the gas amount
            max_fee_per_gas: None,  // TODO fetch the max_fee_per_gas
            max_priority_fee_per_gas,
            input,
            signature,
            chain_id: Some(CHAIN_ID.into()),
            access_list: None,      // TODO fetch the access list
            transaction_type: None, // TODO fetch the transaction type
        })
    }
}

impl StarknetTransaction {
    /// Checks if the transaction is a Kakarot transaction.
    ///
    /// ## Arguments
    ///
    /// * `client` - The Kakarot client.
    ///
    /// ## Returns
    ///
    /// `Ok(bool)` if the operation was successful.
    /// `Err(EthApiError)` if the operation failed.
    async fn is_kakarot_tx(&self, client: &dyn KakarotProvider) -> Result<bool, EthApiError> {
        let starknet_block_latest = StarknetBlockId::Tag(BlockTag::Latest);
        let sender_address: FieldElement = self.sender_address()?.into();

        let class_hash = client.starknet_provider().get_class_hash_at(starknet_block_latest, sender_address).await?;

        Ok(class_hash == client.proxy_account_class_hash())
    }
}
