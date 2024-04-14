use std::sync::Arc;

use async_trait::async_trait;
use ethers::abi::Tokenize;
use ethers::signers::{LocalWallet, Signer};
use ethers_solc::artifacts::CompactContractBytecode;
use reth_primitives::{sign_message, Address, Transaction, TransactionKind, TransactionSigned, TxEip1559, B256, U256};
use starknet::core::types::{MaybePendingTransactionReceipt, TransactionReceipt};
use starknet::core::utils::get_selector_from_name;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::eth_provider::provider::{EthDataProvider, EthereumProvider};
use crate::eth_provider::starknet::kakarot_core::starknet_address;
use crate::models::felt::Felt252Wrapper;
use crate::test_utils::evm_contract::EvmContract;
use crate::test_utils::evm_contract::KakarotEvmContract;
use crate::test_utils::tx_waiter::watch_tx;

pub const TX_GAS_LIMIT: u64 = 5_000_000;

/// EOA is an Ethereum-like Externally Owned Account (EOA) that can sign transactions and send them to the underlying Starknet provider.
#[async_trait]
pub trait Eoa<P: Provider + Send + Sync> {
    fn starknet_address(&self) -> Result<FieldElement, eyre::Error> {
        Ok(starknet_address(self.evm_address()?))
    }
    fn evm_address(&self) -> Result<Address, eyre::Error> {
        let wallet = LocalWallet::from_bytes(self.private_key().as_slice())?;
        Ok(Address::from_slice(wallet.address().as_bytes()))
    }
    fn private_key(&self) -> B256;
    fn eth_provider(&self) -> &EthDataProvider<P>;

    async fn nonce(&self) -> Result<U256, eyre::Error> {
        let eth_provider = self.eth_provider();
        let evm_address = self.evm_address()?;

        Ok(eth_provider.transaction_count(evm_address, None).await?)
    }

    fn sign_transaction(&self, tx: Transaction) -> Result<TransactionSigned, eyre::Error> {
        let pk = self.private_key();
        let signature = sign_message(pk, tx.signature_hash())?;
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

    pub async fn deploy_evm_contract<T: Tokenize>(
        &self,
        contract_name: Option<&str>,
        constructor_args: T,
    ) -> Result<KakarotEvmContract, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;
        let chain_id = self.eth_provider.chain_id().await?.unwrap_or_default();

        // Empty bytecode if contract_name is None
        let bytecode = if let Some(name) = contract_name {
            <KakarotEvmContract as EvmContract>::load_contract_bytecode(name)?
        } else {
            CompactContractBytecode::default()
        };

        let expected_address = {
            let expected_eth_address = self.evm_address().expect("Failed to get EVM address").create(nonce);
            FieldElement::from_byte_slice_be(expected_eth_address.as_slice())
                .expect("Failed to convert address to field element")
        };

        let tx = if contract_name.is_none() {
            Transaction::Eip1559(TxEip1559 {
                chain_id: chain_id.try_into()?,
                nonce,
                gas_limit: TX_GAS_LIMIT,
                ..Default::default()
            })
        } else {
            <KakarotEvmContract as EvmContract>::prepare_create_transaction(
                &bytecode,
                constructor_args,
                nonce,
                chain_id.try_into()?,
            )?
        };
        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;
        let tx_hash: Felt252Wrapper = tx_hash.try_into().expect("Tx Hash should fit into Felt252Wrapper");

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
            .get_transaction_receipt(FieldElement::from(tx_hash))
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
            .find(|event| event.keys.contains(&selector) && event.data.contains(&expected_address))
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
        let nonce: u64 = self.nonce().await?.try_into()?;
        let chain_id = self.eth_provider.chain_id().await?.unwrap_or_default();

        let tx = contract.prepare_call_transaction(function, args, nonce, value, chain_id.try_into()?)?;
        let tx_signed = self.sign_transaction(tx)?;
        let tx_hash = self.send_transaction(tx_signed).await?;

        let bytes = tx_hash.0;
        let starknet_tx_hash = FieldElement::from_bytes_be(&bytes).unwrap();

        watch_tx(self.eth_provider.starknet_provider(), starknet_tx_hash, std::time::Duration::from_millis(300), 60)
            .await
            .expect("Tx polling failed");

        Ok(starknet_tx_hash)
    }

    /// Transfers value to the given address
    /// The transaction is signed and sent by the EOA
    pub async fn transfer(&self, to: Address, value: u128) -> Result<B256, eyre::Error> {
        let tx = Transaction::Eip1559(TxEip1559 {
            chain_id: self.eth_provider.chain_id().await?.unwrap_or_default().try_into()?,
            nonce: self.nonce().await?.try_into()?,
            gas_limit: TX_GAS_LIMIT,
            to: TransactionKind::Call(to),
            value: U256::from(value),
            ..Default::default()
        });

        let tx_signed = self.sign_transaction(tx)?;

        Ok(self.send_transaction(tx_signed).await?)
    }
}
