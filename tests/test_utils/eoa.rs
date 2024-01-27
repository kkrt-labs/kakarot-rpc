use async_trait::async_trait;
use bytes::BytesMut;
use ethers::abi::Tokenize;
use ethers::signers::{LocalWallet, Signer};
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::starknet_client::constants::CHAIN_ID;
use kakarot_rpc::starknet_client::KakarotClient;
use reth_primitives::{
    sign_message, Address, BlockId, BlockNumberOrTag, Bytes, Transaction, TransactionKind, TransactionSigned,
    TxEip1559, H256, U256,
};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, MaybePendingTransactionReceipt, TransactionReceipt};
use starknet::core::utils::get_selector_from_name;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;
use std::sync::Arc;

use crate::test_utils::evm_contract::EvmContract;
use crate::test_utils::evm_contract::KakarotEvmContract;
use crate::test_utils::tx_waiter::watch_tx;

/// EOA is an Ethereum-like Externally Owned Account (EOA) that can sign transactions and send them to the underlying Starknet provider.
#[async_trait]
pub trait Eoa<P: Provider + Send + Sync> {
    async fn starknet_address(&self) -> Result<FieldElement, eyre::Error> {
        let client: &KakarotClient<P> = self.client();
        Ok(client.compute_starknet_address(&self.evm_address()?, &StarknetBlockId::Tag(BlockTag::Latest)).await?)
    }
    fn evm_address(&self) -> Result<Address, eyre::Error> {
        let wallet = LocalWallet::from_bytes(self.private_key().as_bytes())?;
        Ok(Address::from_slice(wallet.address().as_bytes()))
    }
    fn private_key(&self) -> H256;
    fn provider(&self) -> Arc<P>;
    fn client(&self) -> &KakarotClient<P>;

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

    async fn send_transaction(&self, tx: TransactionSigned) -> Result<H256, eyre::Error> {
        let client = self.client();
        let mut buffer = BytesMut::new();
        tx.encode_enveloped(&mut buffer);
        Ok(client.send_transaction(buffer.to_vec().into()).await?)
    }
}

pub struct KakarotEOA<P: Provider + Send + Sync> {
    pub private_key: H256,
    pub client: KakarotClient<P>,
}

impl<P: Provider + Send + Sync> KakarotEOA<P> {
    pub const fn new(private_key: H256, client: KakarotClient<P>) -> Self {
        Self { private_key, client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> Eoa<P> for KakarotEOA<P> {
    fn private_key(&self) -> H256 {
        self.private_key
    }
    fn provider(&self) -> Arc<P> {
        self.client.starknet_provider()
    }
    fn client(&self) -> &KakarotClient<P> {
        &self.client
    }
}

impl<P: Provider + Send + Sync> KakarotEOA<P> {
    pub async fn deploy_evm_contract<T: Tokenize>(
        &self,
        contract_name: &str,
        constructor_args: T,
    ) -> Result<KakarotEvmContract, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;

        let bytecode = <KakarotEvmContract as EvmContract>::load_contract_bytecode(contract_name)?;

        let tx = <KakarotEvmContract as EvmContract>::prepare_create_transaction(&bytecode, constructor_args, nonce)?;
        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;
        let tx_hash: Felt252Wrapper = tx_hash.try_into().expect("Tx Hash should fit into Felt252Wrapper");

        watch_tx(self.provider(), tx_hash.clone().into(), std::time::Duration::from_millis(100))
            .await
            .expect("Tx polling failed");

        let maybe_receipt = self
            .provider()
            .get_transaction_receipt(FieldElement::from(tx_hash.clone()))
            .await
            .expect("Failed to get transaction receipt after retries");

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

    /// Calls a KakarotEvmContract function and returns the Starknet transaction hash
    /// The transaction is signed and sent by the EOA
    /// The transaction is waited for until it is confirmed
    ///
    /// allow(dead_code) is used because this function is used in tests,
    /// and each test is compiled separately, so the compiler thinks this function is unused
    #[allow(dead_code)]
    pub async fn call_evm_contract<T: Tokenize>(
        &self,
        contract: &KakarotEvmContract,
        function: &str,
        args: T,
        value: u128,
    ) -> Result<FieldElement, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;

        let tx = contract.prepare_call_transaction(function, args, nonce, value)?;
        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;

        let bytes = tx_hash.to_fixed_bytes();
        let starknet_tx_hash = FieldElement::from_bytes_be(&bytes).unwrap();

        watch_tx(self.provider(), starknet_tx_hash, std::time::Duration::from_millis(100))
            .await
            .expect("Tx polling failed");

        Ok(starknet_tx_hash)
    }

    /// Transfers value to the given address
    /// The transaction is signed and sent by the EOA
    ///
    /// allow(dead_code) is used because this function is used in tests,
    /// and each test is compiled separately, so the compiler thinks this function is unused
    #[allow(dead_code)]
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

        Ok(tx_hash)
    }
}
