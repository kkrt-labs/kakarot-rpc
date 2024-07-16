use crate::{
    eth_provider::{
        provider::{EthDataProvider, EthereumProvider},
        starknet::kakarot_core::starknet_address,
    },
    models::felt::Felt252Wrapper,
    test_utils::{
        evm_contract::{EvmContract, KakarotEvmContract, TransactionInfo, TxCommonInfo, TxFeeMarketInfo},
        tx_waiter::watch_tx,
    },
};
use alloy_dyn_abi::DynSolValue;
use alloy_json_abi::ContractObject;
use alloy_signer_wallet::LocalWallet;
use async_trait::async_trait;
use reth_primitives::{
    sign_message, Address, Transaction, TransactionSigned, TransactionSignedEcRecovered, TxEip1559, TxKind, B256, U256,
};
use reth_rpc_types_compat::transaction::from_recovered;
use starknet::{
    core::{
        types::{Felt, TransactionReceipt},
        utils::get_selector_from_name,
    },
    providers::Provider,
};
use std::sync::Arc;

pub const TX_GAS_LIMIT: u64 = 5_000_000;
pub const TX_GAS_PRICE: u128 = 10;

/// EOA is an Ethereum-like Externally Owned Account (EOA) that can sign transactions and send them to the underlying Starknet provider.
#[async_trait]
pub trait Eoa<P: Provider + Send + Sync> {
    fn starknet_address(&self) -> Result<Felt, eyre::Error> {
        Ok(starknet_address(self.evm_address()?))
    }
    fn evm_address(&self) -> Result<Address, eyre::Error> {
        let wallet = LocalWallet::from_bytes(&self.private_key())?;
        Ok(wallet.address())
    }
    fn private_key(&self) -> B256;
    fn eth_provider(&self) -> &EthDataProvider<P>;

    async fn nonce(&self) -> Result<U256, eyre::Error> {
        let eth_provider = self.eth_provider();
        let evm_address = self.evm_address()?;

        Ok(eth_provider.transaction_count(evm_address, None).await?)
    }

    fn sign_payload(&self, payload: B256) -> Result<reth_primitives::Signature, eyre::Error> {
        let pk = self.private_key();
        let signature = sign_message(pk, payload)?;
        Ok(signature)
    }

    fn sign_transaction(&self, tx: Transaction) -> Result<TransactionSigned, eyre::Error> {
        let signature = self.sign_payload(tx.signature_hash())?;
        Ok(TransactionSigned::from_transaction_and_signature(tx, signature))
    }

    async fn send_transaction(&self, tx: TransactionSigned) -> Result<B256, eyre::Error> {
        let eth_provider = self.eth_provider();
        let mut v = Vec::new();
        tx.encode_enveloped(&mut v);
        Ok(eth_provider.send_raw_transaction(v.into()).await?)
    }
}

#[derive(Clone, Debug)]
pub struct KakarotEOA<P: Provider> {
    pub private_key: B256,
    pub eth_provider: Arc<EthDataProvider<P>>,
}

impl<P: Provider> KakarotEOA<P> {
    pub const fn new(private_key: B256, eth_provider: Arc<EthDataProvider<P>>) -> Self {
        Self { private_key, eth_provider }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> Eoa<P> for KakarotEOA<P> {
    fn private_key(&self) -> B256 {
        self.private_key
    }

    fn eth_provider(&self) -> &EthDataProvider<P> {
        &self.eth_provider
    }
}

impl<P: Provider + Send + Sync> KakarotEOA<P> {
    fn starknet_provider(&self) -> &P {
        self.eth_provider.starknet_provider()
    }

    /// Deploys an EVM contract given a contract name and constructor arguments
    /// Returns a `KakarotEvmContract` instance
    pub async fn deploy_evm_contract(
        &self,
        contract_name: Option<&str>,
        constructor_args: &[DynSolValue],
    ) -> Result<KakarotEvmContract, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;
        let chain_id = self.eth_provider.chain_id().await?.unwrap_or_default();

        // Empty bytecode if contract_name is None
        let bytecode = if let Some(name) = contract_name {
            <KakarotEvmContract as EvmContract>::load_contract_bytecode(name)?
        } else {
            ContractObject::default()
        };

        let expected_address = {
            let expected_eth_address = self.evm_address().expect("Failed to get EVM address").create(nonce);
            Felt::from_bytes_be_slice(expected_eth_address.as_slice())
        };

        let tx = if contract_name.is_none() {
            Transaction::Eip1559(TxEip1559 {
                chain_id: chain_id.try_into()?,
                nonce,
                gas_limit: TX_GAS_LIMIT,
                max_fee_per_gas: TX_GAS_PRICE,
                ..Default::default()
            })
        } else {
            <KakarotEvmContract as EvmContract>::prepare_create_transaction(
                &bytecode,
                constructor_args,
                &TxCommonInfo { nonce, chain_id: Some(chain_id.try_into()?), ..Default::default() },
            )?
        };
        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;
        let tx_hash: Felt252Wrapper = tx_hash.into();

        watch_tx(
            self.eth_provider.starknet_provider(),
            tx_hash.clone().into(),
            std::time::Duration::from_millis(300),
            60,
        )
        .await
        .expect("Tx polling failed");

        let maybe_receipt = self
            .starknet_provider()
            .get_transaction_receipt(Felt::from(tx_hash))
            .await
            .expect("Failed to get transaction receipt after retries");

        let TransactionReceipt::Invoke(receipt) = maybe_receipt.receipt else {
            return Err(eyre::eyre!("Failed to deploy contract"));
        };

        let selector = get_selector_from_name("evm_contract_deployed").unwrap(); // safe unwrap

        let event = receipt
            .events
            .into_iter()
            .find(|event| event.keys.contains(&selector) && event.data.contains(&expected_address))
            .ok_or_else(|| eyre::eyre!("Failed to find deployed contract address"))?;

        Ok(KakarotEvmContract::new(bytecode, event.data[1], event.data[0]))
    }

    /// Calls a `KakarotEvmContract` function and returns the Starknet transaction hash
    /// The transaction is signed and sent by the EOA
    /// The transaction is waited for until it is confirmed
    pub async fn call_evm_contract(
        &self,
        contract: &KakarotEvmContract,
        function: &str,
        args: &[DynSolValue],
        value: u128,
    ) -> Result<Transaction, eyre::Error> {
        let nonce = self.nonce().await?.try_into()?;
        let chain_id = self.eth_provider.chain_id().await?.unwrap_or_default().to();

        let tx = contract.prepare_call_transaction(
            function,
            args,
            &TransactionInfo::FeeMarketInfo(TxFeeMarketInfo {
                common: TxCommonInfo { chain_id: Some(chain_id), nonce, value },
                ..Default::default()
            }),
        )?;
        let tx_signed = self.sign_transaction(tx.clone())?;
        let tx_hash = self.send_transaction(tx_signed).await?;

        let bytes = tx_hash.0;
        let starknet_tx_hash = Felt::from_bytes_be(&bytes);

        watch_tx(self.eth_provider.starknet_provider(), starknet_tx_hash, std::time::Duration::from_millis(300), 60)
            .await
            .expect("Tx polling failed");

        Ok(tx)
    }

    /// Transfers value to the given address
    /// The transaction is signed and sent by the EOA
    pub async fn transfer(&self, to: Address, value: u128) -> Result<Transaction, eyre::Error> {
        let tx = Transaction::Eip1559(TxEip1559 {
            chain_id: self.eth_provider.chain_id().await?.unwrap_or_default().try_into()?,
            nonce: self.nonce().await?.try_into()?,
            gas_limit: TX_GAS_LIMIT,
            max_fee_per_gas: TX_GAS_PRICE,
            to: TxKind::Call(to),
            value: U256::from(value),
            ..Default::default()
        });

        let tx_signed = self.sign_transaction(tx.clone())?;

        let _ = self.send_transaction(tx_signed).await;
        Ok(tx)
    }

    /// Mocks a transaction with the given nonce without executing it
    pub async fn mock_transaction_with_nonce(&self, nonce: u64) -> Result<reth_rpc_types::Transaction, eyre::Error> {
        let chain_id = self.eth_provider.chain_id().await?.unwrap_or_default().to();
        Ok(from_recovered(TransactionSignedEcRecovered::from_signed_transaction(
            self.sign_transaction(Transaction::Eip1559(TxEip1559 {
                chain_id,
                nonce,
                gas_limit: 21000,
                to: TxKind::Call(Address::random()),
                value: U256::from(1000),
                max_fee_per_gas: TX_GAS_PRICE,
                ..Default::default()
            }))?,
            self.evm_address()?,
        )))
    }
}
