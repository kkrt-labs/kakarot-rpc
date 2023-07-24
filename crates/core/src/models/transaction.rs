use async_trait::async_trait;
use reth_primitives::{TransactionSigned, H256, U256};
use reth_rpc_types::{Signature, Transaction as EthTransaction};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, FieldElement, InvokeTransaction, Transaction};
use starknet::providers::Provider;

use super::felt::Felt252Wrapper;
use super::ConversionError;
use crate::client::api::KakarotEthApi;
use crate::client::constants::{self, CHAIN_ID};
use crate::client::errors::EthApiError;
use crate::models::call::Calls;
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
        pub fn $field_v1(&self) -> Result<$type, ConversionError<()>> {
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
    async fn to_eth_transaction<P: Provider + Send + Sync>(
        &self,
        client: &dyn KakarotEthApi<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError<P::Error>> {
        if !self.is_kakarot_tx(client).await? {
            return Err(EthApiError::KakarotDataFilteringError("Transaction".into()));
        }

        let starknet_block_latest = StarknetBlockId::Tag(BlockTag::Latest);
        let sender_address: FieldElement = self.sender_address()?.into();

        let hash: H256 = self.transaction_hash()?.into();

        let nonce: U256 = self.nonce()?.into();

        let from = client.get_evm_address(&sender_address, &starknet_block_latest).await?;

        let max_priority_fee_per_gas = Some(client.max_priority_fee_per_gas());

        let calls: Calls = self.calldata()?.try_into()?;
        let tx: TransactionSigned = (&calls).try_into()?;
        let input = tx.input().to_owned();
        let signature = tx.signature;
        let to = tx.to();

        let v = if signature.odd_y_parity { 1 } else { 0 } + 35 + 2 * CHAIN_ID;
        let signature = Some(Signature { r: signature.r, s: signature.s, v: U256::from_limbs_slice(&[v]) });

        Ok(EthTransaction {
            hash,
            nonce,
            block_hash,
            block_number,
            transaction_index,
            from,
            to,                     // TODO fetch the to
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
    async fn is_kakarot_tx<P: Provider + Send + Sync>(
        &self,
        client: &dyn KakarotEthApi<P>,
    ) -> Result<bool, EthApiError<P::Error>> {
        let starknet_block_latest = StarknetBlockId::Tag(BlockTag::Latest);
        let sender_address: FieldElement = self.sender_address()?.into();

        let class_hash = client.starknet_provider().get_class_hash_at(starknet_block_latest, sender_address).await?;

        Ok(class_hash == client.proxy_account_class_hash())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::client::tests::init_mock_client;
    use crate::mock::constants::{ABDEL_STARKNET_ADDRESS_HEX, PROXY_ACCOUNT_CLASS_HASH_HEX};
    use crate::mock::mock_starknet::{fixtures, AvailableFixtures};

    #[tokio::test]
    async fn test_is_kakarot_tx() {
        // Given
        let starknet_transaction: Transaction =
            serde_json::from_str(include_str!("test_data/conversion/starknet/transaction.json")).unwrap();
        let starknet_transaction: StarknetTransaction = starknet_transaction.into();

        let fixtures = fixtures(vec![AvailableFixtures::GetClassHashAt(
            ABDEL_STARKNET_ADDRESS_HEX.into(),
            PROXY_ACCOUNT_CLASS_HASH_HEX.into(),
        )]);
        let client = init_mock_client(Some(fixtures));

        // When
        let is_kakarot_tx = starknet_transaction.is_kakarot_tx(&client).await.unwrap();

        // Then
        assert!(is_kakarot_tx);
    }

    #[tokio::test]
    async fn test_to_eth_transaction() {
        // Given
        let starknet_transaction: Transaction =
            serde_json::from_str(include_str!("test_data/conversion/starknet/transaction.json")).unwrap();
        let starknet_transaction: StarknetTransaction = starknet_transaction.into();

        let fixtures = fixtures(vec![
            AvailableFixtures::GetClassHashAt(ABDEL_STARKNET_ADDRESS_HEX.into(), PROXY_ACCOUNT_CLASS_HASH_HEX.into()),
            AvailableFixtures::GetEvmAddress,
        ]);
        let client = init_mock_client(Some(fixtures));

        // When
        let eth_transaction = starknet_transaction.to_eth_transaction(&client, None, None, None).await.unwrap();

        // Then
        let expected: EthTransaction =
            serde_json::from_str(include_str!("test_data/conversion/eth/transaction.json")).unwrap();
        assert_eq!(expected, eth_transaction);
    }
}
