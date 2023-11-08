use std::sync::Arc;

use async_trait::async_trait;
use bytes::BytesMut;
use ethers::abi::Tokenize;
use ethers::signers::{LocalWallet, Signer};
use kakarot_rpc_core::client::api::{KakarotEthApi, KakarotStarknetApi};
use kakarot_rpc_core::client::constants::CHAIN_ID;
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::models::felt::Felt252Wrapper;
use reth_primitives::{
    sign_message, Address, BlockId, BlockNumberOrTag, Bytes, Transaction, TransactionKind, TransactionSigned,
    TxEip1559, H256, U256,
};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, MaybePendingTransactionReceipt, TransactionReceipt};
use starknet::core::utils::get_selector_from_name;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use super::contract::KakarotEvmContract;
use crate::execution::contract::EvmContract;

#[async_trait]
pub trait EOA<P: Provider + Send + Sync + 'static> {
    async fn starknet_address(&self) -> Result<FieldElement, eyre::Error> {
        let client = self.client();
        Ok(client.compute_starknet_address(self.evm_address()?, &StarknetBlockId::Tag(BlockTag::Latest)).await?)
    }
    fn evm_address(&self) -> Result<Address, eyre::Error> {
        let wallet = LocalWallet::from_bytes(self.private_key().as_bytes())?;
        Ok(Address::from_slice(wallet.address().as_bytes()))
    }
    fn private_key(&self) -> H256;
    fn provider(&self) -> Arc<P>;
    fn client(&self) -> &dyn KakarotEthApi<P>;

    async fn nonce(&self) -> Result<U256, eyre::Error> {
        let client = self.client();
        let evm_address = self.evm_address()?;

        Ok(client.nonce(evm_address, BlockId::Number(BlockNumberOrTag::Latest)).await?)
    }

    fn sign_transaction(&self, tx: Transaction) -> Result<TransactionSigned, eyre::Error> {
        let pk = self.private_key();
        let signature = sign_message(pk, tx.signature_hash())?;
        Ok(TransactionSigned::from_transaction_and_signature(tx, signature))
    }

    async fn send_transaction(&self, tx: TransactionSigned) -> Result<U256, eyre::Error> {
        let client = self.client();
        let mut buffer = BytesMut::new();
        tx.encode_enveloped(&mut buffer);
        Ok(client.send_transaction(buffer.to_vec().into()).await?.into())
    }
}

pub struct KakarotEOA<P: Provider + Send + Sync> {
    pub private_key: H256,
    pub client: KakarotClient<P>,
}

impl<P: Provider + Send + Sync> KakarotEOA<P> {
    pub fn new(private_key: H256, client: KakarotClient<P>) -> Self {
        Self { private_key, client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> EOA<P> for KakarotEOA<P> {
    fn private_key(&self) -> H256 {
        self.private_key
    }
    fn provider(&self) -> Arc<P> {
        self.client.starknet_provider()
    }
    fn client(&self) -> &dyn KakarotEthApi<P> {
        &self.client
    }
}

impl<P: Provider + Send + Sync + 'static> KakarotEOA<P> {
    pub async fn deploy_evm_contract<T: Tokenize>(
        &self,
        contract_name: &str,
        constructor_args: T,
    ) -> Result<KakarotEvmContract, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;

        let bytecode = <KakarotEvmContract as EvmContract>::load_contract_bytecode(contract_name)?;

        let tx = <KakarotEvmContract as EvmContract>::create_transaction(&bytecode, constructor_args, nonce)?;
        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;
        let tx_hash: Felt252Wrapper = tx_hash.try_into()?;

        let maybe_receipt = self.provider().get_transaction_receipt(FieldElement::from(tx_hash)).await?;

        let receipt = match maybe_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) => receipt,
            _ => return Err(eyre::eyre!("Failed to deploy contract")),
        };

        let selector = get_selector_from_name("evm_contract_deployed").unwrap(); // safe unwrap

        let event = receipt
            .events
            .into_iter()
            .find(|event| event.keys.contains(&selector))
            .ok_or_else(|| eyre::eyre!("Failed to find deployed contract address"))?;

        Ok(KakarotEvmContract::new(bytecode, event.data[1], event.data[0]))
    }

    pub async fn call_evm_contract<T: Tokenize>(
        &self,
        contract: &KakarotEvmContract,
        function: &str,
        args: T,
        value: u128,
    ) -> Result<H256, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;

        let tx = contract.call_transaction(function, args, nonce, value)?;
        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;
        let tx_hash: Felt252Wrapper = tx_hash.try_into()?;

        Ok(tx_hash.into())
    }

    pub async fn transfer(&self, to: Address, value: u128) -> Result<H256, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;

        let tx = Transaction::Eip1559(TxEip1559 {
            chain_id: CHAIN_ID,
            nonce,
            max_priority_fee_per_gas: Default::default(),
            max_fee_per_gas: Default::default(),
            gas_limit: u64::MAX,
            to: TransactionKind::Call(to),
            value,
            input: Bytes::default(),
            access_list: Default::default(),
        });

        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;
        let tx_hash: Felt252Wrapper = tx_hash.try_into()?;

        Ok(tx_hash.into())
    }
}
