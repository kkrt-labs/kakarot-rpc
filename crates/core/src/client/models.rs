use super::{
    client_api::{KakarotClient, KakarotClientError},
    constants::{self, CHAIN_ID},
    convertibles::ConvertibleStarknetTransaction,
    helpers::{decode_signature_from_tx_calldata, vec_felt_to_bytes, DataDecodingError},
};
use async_trait::async_trait;
use reth_primitives::{Address, H256, U256};
use reth_rpc_types::{Signature, Transaction as EthTransaction};
use serde::{Deserialize, Serialize};
use starknet::{
    core::types::{
        BlockId as StarknetBlockId, BlockTag, FieldElement, InvokeTransaction, Transaction,
    },
    providers::Provider,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("transaction conversion error: {0}")]
    TransactionConvertionError(String),
    #[error(transparent)]
    DataDecodingError(#[from] DataDecodingError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    pub contract_address: Address,
    pub token_balance: Option<U256>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    pub address: Address,
    pub token_balances: Vec<TokenBalance>,
}

pub struct Felt252Wrapper(FieldElement);

impl From<FieldElement> for Felt252Wrapper {
    fn from(felt: FieldElement) -> Self {
        Self(felt)
    }
}

impl From<Felt252Wrapper> for FieldElement {
    fn from(felt: Felt252Wrapper) -> Self {
        felt.0
    }
}

impl From<Felt252Wrapper> for H256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        H256::from_slice(&felt.to_bytes_be())
    }
}

impl From<Felt252Wrapper> for U256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        U256::from_be_bytes(felt.to_bytes_be())
    }
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
    (($field_v0: ident, $field_v1: ident), $type: ty) => {
        pub fn $field_v1(&self) -> Option<$type> {
            match &self.0 {
                Transaction::Invoke(tx) => match tx {
                    InvokeTransaction::V0(tx) => Some(tx.$field_v0.clone().into()),
                    InvokeTransaction::V1(tx) => Some(tx.$field_v1.clone().into()),
                },
                _ => None,
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

#[async_trait]
impl ConvertibleStarknetTransaction for StarknetTransaction {
    async fn to_eth_transaction(
        &self,
        client: &dyn KakarotClient,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, KakarotClientError> {
        let starknet_block_latest = StarknetBlockId::Tag(BlockTag::Latest);
        let sender_address: FieldElement = option_to_result(
            self.sender_address(),
            constants::error_messages::INVALID_TRANSACTION_TYPE,
        )?
        .into();

        let class_hash = client
            .inner()
            .get_class_hash_at(starknet_block_latest, sender_address)
            .await?;

        if class_hash != client.proxy_account_class_hash() {
            return Err(KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Filter: Tx is not part of Kakarot"
            )));
        }

        let hash: H256 = option_to_result(
            self.transaction_hash(),
            constants::error_messages::INVALID_TRANSACTION_TYPE,
        )?
        .into();

        let nonce: U256 = option_to_result(
            self.nonce(),
            constants::error_messages::INVALID_TRANSACTION_TYPE,
        )?
        .into();

        let from = client
            .get_evm_address(&sender_address, &starknet_block_latest)
            .await?;

        let max_priority_fee_per_gas = Some(client.max_priority_fee_per_gas());

        let calldata = self.calldata().unwrap_or_default();
        let input = vec_felt_to_bytes(calldata.clone());

        // TODO: wrap to abstract the following lines?
        // Extracting the signature
        let signature = decode_signature_from_tx_calldata(&calldata)?;
        let v = if signature.odd_y_parity { 1 } else { 0 } + 35 + 2 * CHAIN_ID;
        let signature = Some(Signature {
            r: signature.r,
            s: signature.s,
            v: U256::from_limbs_slice(&[v]),
        });

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

pub fn option_to_result<T>(option: Option<T>, message: &str) -> Result<T, ConversionError> {
    option.ok_or_else(|| ConversionError::TransactionConvertionError(message.to_string()))
}
