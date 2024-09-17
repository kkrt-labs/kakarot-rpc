use crate::{
    models::transaction::transaction_data_to_starknet_calldata,
    providers::eth_provider::{
        constant::{CHAIN_ID, RPC_CONFIG},
        error::{SignatureError, TransactionError},
        provider::EthApiResult,
        starknet::kakarot_core::{starknet_address, EXECUTE_FROM_OUTSIDE},
    },
};
use reth_primitives::TransactionSigned;
use starknet::{
    accounts::{Account, ExecutionEncoding, ExecutionV1, SingleOwnerAccount},
    core::types::Felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider},
    signers::{LocalWallet, SigningKey},
};
use std::{env::var, ops::Deref, str::FromStr, sync::LazyLock};
use tokio::sync::MutexGuard;

/// Signer for all relayers
static RELAYER_SIGNER: LazyLock<LocalWallet> = LazyLock::new(|| {
    LocalWallet::from_signing_key(SigningKey::from_secret_scalar(
        Felt::from_str(&var("RELAYER_PRIVATE_KEY").expect("missing relayer private key"))
            .expect("failed to parse relayer private key"),
    ))
});

/// A relayer holding a lock on a mutex on an account and connected to the Starknet network.
/// The relayer is used to sign  transactions and broadcast them on the network.
#[derive(Debug)]
pub struct LockedRelayer<'a, SP: Provider + Send + Sync> {
    /// The account used to sign and broadcast the transaction
    account: SingleOwnerAccount<SP, LocalWallet>,
    /// The balance of the relayer
    balance: Felt,
    /// The locked nonce held by the relayer
    nonce: MutexGuard<'a, Felt>,
}

impl<'a, SP> LockedRelayer<'a, SP>
where
    SP: Provider + Send + Sync,
{
    /// Create a new relayer with the provided Starknet provider, address, balance and nonce.
    pub fn new(lock: MutexGuard<'a, Felt>, address: Felt, balance: Felt, provider: SP, chain_id: Felt) -> Self {
        let relayer =
            SingleOwnerAccount::new(provider, RELAYER_SIGNER.clone(), address, chain_id, ExecutionEncoding::New);

        tracing::info!("balance of the relayer: {:?}", balance);

        tracing::info!("chain id to create the relayer: {:?}", chain_id);
        Self { account: relayer, balance, nonce: lock }
    }

    /// Relay the provided Ethereum transaction on the Starknet network.
    /// Returns the corresponding Starknet transaction hash.
    pub async fn relay_transaction(&self, transaction: &TransactionSigned) -> EthApiResult<Felt> {
        // Transform the transaction's data to Starknet calldata
        let relayer_address = self.account.address();
        let calldata = transaction_data_to_starknet_calldata(transaction, relayer_address)?;

        // Recover the signer
        let eoa_address = transaction.recover_signer().ok_or(SignatureError::Recovery)?;
        let eoa_address = starknet_address(eoa_address);

        // Construct the call
        let call = starknet::accounts::Call { to: eoa_address, selector: *EXECUTE_FROM_OUTSIDE, calldata };
        let mut execution = ExecutionV1::new(vec![call], &self.account);
        execution = execution.nonce(*self.nonce);

        // We set the max fee to the balance of the account - 1. This might cause some issues in the future
        // and should be replaced by a simulation of the transaction (?)
        execution = execution.max_fee(self.balance - 1);

        let prepared = execution.prepared().map_err(|_| SignatureError::SigningFailure)?;
        let res = prepared
            .send()
            .await
            .inspect_err(|err| tracing::error!(target: "relayer", ?err))
            .map_err(|err| TransactionError::Broadcast(err.into()))?;

        Ok(res.transaction_hash)
    }

    pub fn nonce_mut(&mut self) -> &mut Felt {
        &mut self.nonce
    }
}

impl<'a, SP> Deref for LockedRelayer<'a, SP>
where
    SP: Provider + Send + Sync,
{
    type Target = SingleOwnerAccount<SP, LocalWallet>;

    fn deref(&self) -> &Self::Target {
        &self.account
    }
}
