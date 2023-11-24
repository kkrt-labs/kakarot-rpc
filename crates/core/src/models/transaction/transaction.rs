use async_trait::async_trait;
use reth_primitives::{TransactionSigned, H256, U128, U256, U64};
use reth_rpc_types::{Signature, Transaction as EthTransaction};
use starknet::core::types::{
    BlockId as StarknetBlockId, BlockTag, FieldElement, InvokeTransaction, StarknetError, Transaction,
};
use starknet::providers::{MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage};

use crate::client::constants::{self, CHAIN_ID};
use crate::client::errors::EthApiError;
use crate::client::KakarotClient;
use crate::models::call::Calls;
use crate::models::convertible::ConvertibleStarknetTransaction;
use crate::models::errors::ConversionError;
use crate::models::felt::Felt252Wrapper;

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
    get_invoke_transaction_field!((calldata, calldata), Vec<FieldElement>);
    get_invoke_transaction_field!((contract_address, sender_address), Felt252Wrapper);

    pub fn transaction_hash(&self) -> H256 {
        H256::from_slice(&self.0.transaction_hash().to_bytes_be())
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

#[async_trait]
impl ConvertibleStarknetTransaction for StarknetTransaction {
    async fn to_eth_transaction<P: Provider + Send + Sync + 'static>(
        &self,
        client: &KakarotClient<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError> {
        if !self.is_kakarot_tx(client).await? {
            return Err(EthApiError::KakarotDataFilteringError("Transaction".into()));
        }

        let sender_address: FieldElement = self.sender_address()?.into();

        let hash = self.transaction_hash();

        let starknet_block_id = match block_hash {
            Some(block_hash) => StarknetBlockId::Hash(TryInto::<Felt252Wrapper>::try_into(block_hash)?.into()),
            None => match block_number {
                Some(block_number) => StarknetBlockId::Number(block_number.as_limbs()[0]),
                None => {
                    return Err(EthApiError::RequestError(ProviderError::StarknetError(StarknetErrorWithMessage {
                        code: MaybeUnknownErrorCode::Known(StarknetError::BlockNotFound),
                        message: "Block hash or block number must be provided".into(),
                    })));
                }
            },
        };
        let nonce: Felt252Wrapper = match &self.0 {
            Transaction::Invoke(invoke_tx) => match invoke_tx {
                InvokeTransaction::V0(_) => {
                    client.starknet_provider().get_nonce(starknet_block_id, sender_address).await?.into()
                }
                InvokeTransaction::V1(v1) => v1.nonce.into(),
            },
            _ => return Err(EthApiError::KakarotDataFilteringError("Transaction".into())),
        };
        let nonce: U64 = u64::try_from(nonce)?.into();

        let from = client.get_evm_address(&sender_address).await?;

        let max_priority_fee_per_gas = Some(client.max_priority_fee_per_gas());

        let calls: Calls = self.calldata()?.try_into()?;
        let tx: TransactionSigned = (&calls).try_into()?;
        let input = tx.input().to_owned();
        let signature = tx.signature;
        let to = tx.to();
        let value = U256::from(tx.value());
        let max_fee_per_gas = Some(U128::from(tx.max_fee_per_gas()));
        let transaction_type = Some(U64::from(Into::<u8>::into(tx.tx_type())));

        let v = if signature.odd_y_parity { 1 } else { 0 } + 35 + 2 * CHAIN_ID;
        let signature =
            Some(Signature { r: signature.r, s: signature.s, v: U256::from_limbs_slice(&[v]), y_parity: None });

        Ok(EthTransaction {
            hash,
            nonce,
            block_hash,
            block_number,
            transaction_index,
            from,
            to,
            value,
            gas_price: None,      // TODO fetch the gas price
            gas: U256::from(100), // TODO fetch the gas amount
            max_fee_per_gas,
            max_priority_fee_per_gas,
            input,
            signature,
            chain_id: Some(CHAIN_ID.into()),
            access_list: None, // TODO fetch the access list
            transaction_type,
            max_fee_per_blob_gas: None,
            blob_versioned_hashes: Vec::new(),
        })
    }
}

impl StarknetTransaction {
    /// Checks if the transaction is a Kakarot transaction.
    async fn is_kakarot_tx<P: Provider + Send + Sync + 'static>(
        &self,
        client: &KakarotClient<P>,
    ) -> Result<bool, EthApiError> {
        let starknet_block_latest = StarknetBlockId::Tag(BlockTag::Latest);
        let sender_address: FieldElement = self.sender_address()?.into();

        let class_hash = client.starknet_provider().get_class_hash_at(starknet_block_latest, sender_address).await?;

        Ok(class_hash == client.proxy_account_class_hash())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::mock::constants::{ABDEL_STARKNET_ADDRESS_HEX, PROXY_ACCOUNT_CLASS_HASH_HEX};
    use crate::mock::mock_starknet::{fixtures, init_mock_client, AvailableFixtures};

    #[tokio::test]
    async fn test_is_kakarot_tx() {
        // Given
        let starknet_transaction: Transaction =
            serde_json::from_str(include_str!("../test_data/conversion/starknet/transaction.json")).unwrap();
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
            serde_json::from_str(include_str!("../test_data/conversion/starknet/transaction.json")).unwrap();
        let starknet_transaction: StarknetTransaction = starknet_transaction.into();

        let fixtures = fixtures(vec![
            AvailableFixtures::GetClassHashAt(ABDEL_STARKNET_ADDRESS_HEX.into(), PROXY_ACCOUNT_CLASS_HASH_HEX.into()),
            AvailableFixtures::GetEvmAddress,
        ]);
        let client = init_mock_client(Some(fixtures));

        // When
        let eth_transaction =
            starknet_transaction.to_eth_transaction(&client, None, Some(U256::from(1234u64)), None).await.unwrap();

        // Then
        let expected: EthTransaction =
            serde_json::from_str(include_str!("../test_data/conversion/eth/transaction.json")).unwrap();
        assert_eq!(expected, eth_transaction);
    }
}
