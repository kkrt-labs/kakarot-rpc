use crate::{
    client::{EthClient, KakarotTransactions},
    into_via_try_wrapper,
    providers::eth_provider::{
        starknet::{kakarot_core::starknet_address, relayer::LockedRelayer},
        ChainProvider, TransactionProvider,
    },
    test_utils::{
        evm_contract::{EvmContract, KakarotEvmContract, TransactionInfo, TxCommonInfo, TxFeeMarketInfo},
        tx_waiter::watch_tx,
    },
};
use alloy_dyn_abi::DynSolValue;
use alloy_json_abi::ContractObject;
use alloy_signer_local::PrivateKeySigner;
use async_trait::async_trait;
use reth_primitives::{
    sign_message, Address, Transaction, TransactionSigned, TransactionSignedEcRecovered, TxEip1559, TxKind, B256, U256,
};
use reth_rpc_types_compat::transaction::from_recovered;
use starknet::{
    accounts::{Account, SingleOwnerAccount},
    core::{
        types::{BlockId, BlockTag, Felt, TransactionReceipt},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
    signers::LocalWallet,
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const TX_GAS_LIMIT: u64 = 5_000_000;
pub const TX_GAS_PRICE: u128 = 10;

/// EOA is an Ethereum-like Externally Owned Account (EOA) that can sign transactions and send them to the underlying Starknet provider.
#[async_trait]
pub trait Eoa<P: Provider + Send + Sync + Clone> {
    fn starknet_address(&self) -> Result<Felt, eyre::Error> {
        Ok(starknet_address(self.evm_address()?))
    }

    fn evm_address(&self) -> Result<Address, eyre::Error> {
        let wallet = PrivateKeySigner::from_bytes(&self.private_key())?;
        Ok(wallet.address())
    }

    fn private_key(&self) -> B256;

    fn eth_client(&self) -> &EthClient<P>;

    async fn nonce(&self) -> Result<U256, eyre::Error> {
        let eth_provider = self.eth_client().eth_provider();
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
        let eth_client = self.eth_client();
        let mut v = Vec::new();
        tx.encode_enveloped(&mut v);
        Ok(eth_client.send_raw_transaction(v.into()).await?)
    }
}

#[derive(Clone, Debug)]
pub struct KakarotEOA<P: Provider + Send + Sync + Clone + 'static> {
    pub private_key: B256,
    pub eth_client: Arc<EthClient<P>>,
}

impl<P: Provider + Send + Sync + Clone> KakarotEOA<P> {
    pub const fn new(private_key: B256, eth_client: Arc<EthClient<P>>) -> Self {
        Self { private_key, eth_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + Clone> Eoa<P> for KakarotEOA<P> {
    fn private_key(&self) -> B256 {
        self.private_key
    }

    fn eth_client(&self) -> &EthClient<P> {
        &self.eth_client
    }
}

impl<P: Provider + Send + Sync + Clone> KakarotEOA<P> {
    fn starknet_provider(&self) -> &P {
        self.eth_client.starknet_provider()
    }

    /// Deploys an EVM contract given a contract name and constructor arguments
    /// Returns a `KakarotEvmContract` instance
    pub async fn deploy_evm_contract(
        &self,
        contract_name: Option<&str>,
        constructor_args: &[DynSolValue],
        relayer: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    ) -> Result<KakarotEvmContract, eyre::Error> {
        let nonce = self.nonce().await?;
        let nonce: u64 = nonce.try_into()?;
        let chain_id: u64 = self.eth_client.eth_provider().chain_id().await?.unwrap_or_default().try_into()?;

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
                chain_id,
                nonce,
                gas_limit: TX_GAS_LIMIT,
                max_fee_per_gas: TX_GAS_PRICE,
                ..Default::default()
            })
        } else {
            <KakarotEvmContract as EvmContract>::prepare_create_transaction(
                &bytecode,
                constructor_args,
                &TxCommonInfo { nonce, chain_id: Some(chain_id), ..Default::default() },
            )?
        };
        let tx_signed = self.sign_transaction(tx)?;
        let _ = self.send_transaction(tx_signed.clone()).await?;

        // Prepare the relayer
        let relayer_balance =
            self.eth_client.starknet_provider().balance_at(relayer.address(), BlockId::Tag(BlockTag::Latest)).await?;
        let relayer_balance = into_via_try_wrapper!(relayer_balance)?;

        let nonce = self
            .eth_client
            .starknet_provider()
            .get_nonce(BlockId::Tag(BlockTag::Latest), relayer.address())
            .await
            .unwrap_or_default();

        let current_nonce = Mutex::new(nonce);

        // Relay the transaction
        let starknet_transaction_hash = LockedRelayer::new(
            current_nonce.lock().await,
            relayer.address(),
            relayer_balance,
            self.starknet_provider(),
            self.starknet_provider().chain_id().await.expect("Failed to get chain id"),
        )
        .relay_transaction(&tx_signed)
        .await
        .expect("Failed to relay transaction");

        watch_tx(
            self.eth_client.eth_provider().starknet_provider_inner(),
            starknet_transaction_hash,
            std::time::Duration::from_millis(300),
            60,
        )
        .await
        .expect("Tx polling failed");

        let maybe_receipt = self
            .starknet_provider()
            .get_transaction_receipt(starknet_transaction_hash)
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
        relayer: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    ) -> Result<Transaction, eyre::Error> {
        let nonce = self.nonce().await?.try_into()?;
        let chain_id = self.eth_client.eth_provider().chain_id().await?.unwrap_or_default().to();

        let tx = contract.prepare_call_transaction(
            function,
            args,
            &TransactionInfo::FeeMarketInfo(TxFeeMarketInfo {
                common: TxCommonInfo { chain_id: Some(chain_id), nonce, value },
                max_fee_per_gas: 1000,
                max_priority_fee_per_gas: 1000,
            }),
        )?;
        let tx_signed = self.sign_transaction(tx.clone())?;
        let _ = self.send_transaction(tx_signed.clone()).await?;

        // Prepare the relayer
        let relayer_balance =
            self.eth_client.starknet_provider().balance_at(relayer.address(), BlockId::Tag(BlockTag::Latest)).await?;
        let relayer_balance = into_via_try_wrapper!(relayer_balance)?;

        let nonce = self
            .eth_client
            .starknet_provider()
            .get_nonce(BlockId::Tag(BlockTag::Latest), relayer.address())
            .await
            .unwrap_or_default();

        let current_nonce = Mutex::new(nonce);

        // Relay the transaction
        let starknet_transaction_hash = LockedRelayer::new(
            current_nonce.lock().await,
            relayer.address(),
            relayer_balance,
            self.starknet_provider(),
            self.starknet_provider().chain_id().await.expect("Failed to get chain id"),
        )
        .relay_transaction(&tx_signed)
        .await
        .expect("Failed to relay transaction");

        watch_tx(
            self.eth_client.eth_provider().starknet_provider_inner(),
            starknet_transaction_hash,
            std::time::Duration::from_millis(300),
            60,
        )
        .await
        .expect("Tx polling failed");

        Ok(tx)
    }

    /// Transfers value to the given address
    /// The transaction is signed and sent by the EOA
    pub async fn transfer(&self, to: Address, value: u128) -> Result<Transaction, eyre::Error> {
        let tx = Transaction::Eip1559(TxEip1559 {
            chain_id: self.eth_client.eth_provider().chain_id().await?.unwrap_or_default().try_into()?,
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
        let chain_id = self.eth_client.eth_provider().chain_id().await?.unwrap_or_default().to();
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
