use std::sync::Arc;

use reth_primitives::Bytes;
use starknet::accounts::{Account, AccountError, Call, SingleOwnerAccount};
use starknet::core::types::{BlockId, FunctionCall};
use starknet::providers::Provider;
use starknet::signers::LocalWallet;
use starknet_crypto::FieldElement;

use crate::client::constants::selectors::{COMPUTE_STARKNET_ADDRESS, DEPLOY_EXTERNALLY_OWNED_ACCOUNT, ETH_CALL};
use crate::client::errors::EthApiError;
use crate::client::helpers::{decode_eth_call_return, vec_felt_to_bytes, DataDecodingError};
use crate::client::waiter::TransactionWaiter;

pub struct KakarotContract<P> {
    pub address: FieldElement,
    pub proxy_account_class_hash: FieldElement,
    provider: Arc<P>,
}

impl<P: Provider + Send + Sync + 'static> KakarotContract<P> {
    pub fn new(provider: Arc<P>, address: FieldElement, proxy_account_class_hash: FieldElement) -> Self {
        Self { address, proxy_account_class_hash, provider }
    }

    pub async fn compute_starknet_address(
        &self,
        eth_address: &FieldElement,
        block_id: &BlockId,
    ) -> Result<FieldElement, EthApiError<P::Error>> {
        // Prepare the calldata for the compute_starknet_address function call
        let calldata = vec![*eth_address];
        let request =
            FunctionCall { contract_address: self.address, entry_point_selector: COMPUTE_STARKNET_ADDRESS, calldata };

        // Make the function call to get the Starknet contract address
        let result = self.provider.call(request, block_id).await?;
        match result.first() {
            Some(x) if result.len() == 1 => Ok(*x),
            _ => Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "compute_starknet_address".into(),
                expected: 1,
                actual: 0,
            }
            .into()),
        }
    }

    pub async fn eth_call(
        &self,
        origin: &FieldElement,
        to: &FieldElement,
        mut eth_calldata: Vec<FieldElement>,
        block_id: &BlockId,
    ) -> Result<Bytes, EthApiError<P::Error>> {
        let mut calldata =
            vec![*origin, *to, FieldElement::MAX, FieldElement::ZERO, FieldElement::ZERO, eth_calldata.len().into()];

        calldata.append(&mut eth_calldata);

        let request = FunctionCall { contract_address: self.address, entry_point_selector: ETH_CALL, calldata };
        let result = self.provider.call(request, block_id).await?;

        // Parse and decode Kakarot's call return data (temporary solution and not scalable - will
        // fail is Kakarot API changes)
        // Declare Vec of Result
        // TODO: Change to decode based on ABI or use starknet-rs future feature to decode return
        // params
        let return_data = decode_eth_call_return(&result)?;

        let result = vec_felt_to_bytes(return_data);
        Ok(result)
    }

    pub async fn deploy_externally_owned_account(
        &self,
        ethereum_address: FieldElement,
        deployer_account: &SingleOwnerAccount<Arc<P>, LocalWallet>,
    ) -> Result<FieldElement, EthApiError<P::Error>> {
        let result = deployer_account
            .execute(vec![Call {
                calldata: vec![ethereum_address],
                to: self.address,
                selector: DEPLOY_EXTERNALLY_OWNED_ACCOUNT,
            }])
            .send()
            .await;

        let result: Result<FieldElement, EthApiError<P::Error>> = match result {
            Ok(invoke_result) => {
                let waiter =
                    TransactionWaiter::new(self.provider.clone(), invoke_result.transaction_hash, 1000, 15_000);
                waiter.poll().await?;
                Ok(invoke_result.transaction_hash)
            }
            Err(error) => match error {
                AccountError::Provider(error) => Err(EthApiError::from(error)),
                _ => Err(EthApiError::Other(error.into())),
            },
        };

        result
    }
}
