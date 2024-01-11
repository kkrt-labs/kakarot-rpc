use reth_primitives::{Transaction as TransactionType, H256, U128, U256, U64};
use reth_rpc_types::{Signature, Transaction as EthTransaction};
use starknet::core::types::{BlockId as StarknetBlockId, FieldElement, InvokeTransaction, StarknetError, Transaction};
use starknet::providers::{MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage};

use crate::models::call::{Call, Calls};
use crate::models::errors::ConversionError;
use crate::models::felt::Felt252Wrapper;
use crate::models::signature::StarknetSignature;
use crate::starknet_client::constants::CHAIN_ID;
use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::KakarotClient;

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

macro_rules! invoke_transaction_field {
    (($field_v0:ident, $field_v1:ident), $type:ty) => {
        pub fn $field_v1(&self) -> Result<$type, ConversionError> {
            match &self.0 {
                Transaction::Invoke(tx) => match tx {
                    InvokeTransaction::V0(tx) => Ok(tx.$field_v0.clone().into()),
                    InvokeTransaction::V1(tx) => Ok(tx.$field_v1.clone().into()),
                },
                _ => Err(ConversionError::TransactionConversionError(
                    "L1Handler, Declare, Deploy and DeployAccount transactions unsupported".to_string(),
                )),
            }
        }
    };
}

impl StarknetTransaction {
    invoke_transaction_field!((calldata, calldata), Vec<FieldElement>);
    invoke_transaction_field!((contract_address, sender_address), Felt252Wrapper);
    invoke_transaction_field!((signature, signature), Vec<FieldElement>);

    pub fn transaction_hash(&self) -> H256 {
        H256::from_slice(&self.0.transaction_hash().to_bytes_be())
    }
}

impl StarknetTransaction {
    pub async fn to_eth_transaction<P: Provider + Send + Sync>(
        &self,
        client: &KakarotClient<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError> {
        let sender_address: FieldElement = self.sender_address()?.into();

        let hash = self.transaction_hash();

        let starknet_block_id = match block_hash {
            Some(block_hash) => StarknetBlockId::Hash(TryInto::<Felt252Wrapper>::try_into(block_hash)?.into()),
            None => match block_number {
                Some(block_number) => StarknetBlockId::Number(TryInto::<u64>::try_into(block_number)?),
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

        if calls.len() != 1 {
            return Err(EthApiError::ConversionError("Call length is {calls.len()}, expected 1".to_string()));
        }

        let call =
            calls.get(0).ok_or(EthApiError::ConversionError("Call array length != 1 is not supported".to_string()))?;

        let tx: TransactionType = Call::from(call.clone()).try_into()?;
        let input = tx.input().to_owned();
        let signature: Signature = StarknetSignature::from(self.signature()?)
            .try_into()
            .map_err(|_| EthApiError::KakarotDataFilteringError("Transaction Signature".into()))?;
        let to = tx.to();
        let value = U256::from(tx.value());
        let max_fee_per_gas = Some(U128::from(tx.max_fee_per_gas()));
        let transaction_type = Some(U64::from(Into::<u8>::into(tx.tx_type())));

        let signature = Some(signature);

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
