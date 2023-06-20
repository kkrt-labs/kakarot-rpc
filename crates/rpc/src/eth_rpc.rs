use jsonrpsee::core::{async_trait, RpcResult as Result};
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::CallError;
use kakarot_rpc_core::client::client_api::KakarotClient;
use kakarot_rpc_core::client::constants::CHAIN_ID;
use kakarot_rpc_core::client::helpers::ethers_block_id_to_starknet_block_id;
use kakarot_rpc_core::client::models::TokenBalances;
use reth_primitives::rpc::transaction::eip2930::AccessListWithGasUsed;
use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, H256, H64, U128, U256, U64};
use reth_rpc_types::{
    CallRequest, EIP1186AccountProofResponse, FeeHistory, Index, RichBlock, SyncStatus,
    Transaction as EtherTransaction, TransactionReceipt, TransactionRequest, Work,
};
use serde_json::Value;

use crate::eth_api::EthApiServer;

/// The RPC module for the Ethereum protocol required by Kakarot.
pub struct KakarotEthRpc {
    pub kakarot_client: Box<dyn KakarotClient>,
}

#[async_trait]
impl EthApiServer for KakarotEthRpc {
    async fn block_number(&self) -> Result<U64> {
        let block_number = self.kakarot_client.block_number().await?;
        Ok(block_number)
    }

    /// Get the protocol version of the Kakarot Starknet RPC.
    ///
    /// # Returns
    /// * `protocol_version(u64)` - The protocol version.
    ///
    /// `Ok(protocol_version)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    fn protocol_version(&self) -> Result<U64> {
        let protocol_version = 1_u64;
        Ok(protocol_version.into())
    }

    async fn syncing(&self) -> Result<SyncStatus> {
        let status = self.kakarot_client.syncing().await?;
        Ok(status)
    }

    async fn author(&self) -> Result<Address> {
        todo!()
    }

    async fn accounts(&self) -> Result<Vec<Address>> {
        Ok(Vec::new())
    }

    async fn chain_id(&self) -> Result<Option<U64>> {
        // CHAIN_ID = KKRT (0x4b4b5254) in ASCII
        Ok(Some(CHAIN_ID.into()))
    }

    async fn block_by_hash(&self, _hash: H256, _full: bool) -> Result<Option<RichBlock>> {
        let block_id = BlockId::Hash(_hash.into());
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let block = self.kakarot_client.get_eth_block_from_starknet_block(starknet_block_id, _full).await?;
        Ok(Some(block))
    }

    async fn block_by_number(&self, _number: BlockNumberOrTag, _full: bool) -> Result<Option<RichBlock>> {
        let block_id = BlockId::Number(_number);
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let block = self.kakarot_client.get_eth_block_from_starknet_block(starknet_block_id, _full).await?;
        Ok(Some(block))
    }

    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64> {
        let transaction_count = self.kakarot_client.block_transaction_count_by_hash(hash).await?;
        Ok(transaction_count)
    }

    async fn block_transaction_count_by_number(&self, _number: BlockNumberOrTag) -> Result<U64> {
        let transaction_count = self.kakarot_client.block_transaction_count_by_number(_number).await?;
        Ok(transaction_count)
    }

    async fn block_uncles_count_by_hash(&self, _hash: H256) -> Result<U256> {
        todo!()
    }

    async fn block_uncles_count_by_number(&self, _number: BlockNumberOrTag) -> Result<U256> {
        todo!()
    }

    async fn uncle_by_block_hash_and_index(&self, _hash: H256, _index: Index) -> Result<Option<RichBlock>> {
        todo!()
    }

    async fn uncle_by_block_number_and_index(
        &self,
        _number: BlockNumberOrTag,
        _index: Index,
    ) -> Result<Option<RichBlock>> {
        todo!()
    }

    async fn transaction_by_hash(&self, _hash: H256) -> Result<Option<EtherTransaction>> {
        let ether_tx = EtherTransaction::default();

        Ok(Some(ether_tx))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        _hash: H256,
        _index: Index,
    ) -> Result<Option<EtherTransaction>> {
        let block_id = BlockId::Hash(_hash.into());
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let tx = self.kakarot_client.transaction_by_block_id_and_index(starknet_block_id, _index).await?;
        Ok(Some(tx))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        _number: BlockNumberOrTag,
        _index: Index,
    ) -> Result<Option<EtherTransaction>> {
        let block_id = BlockId::Number(_number);
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let tx = self.kakarot_client.transaction_by_block_id_and_index(starknet_block_id, _index).await?;
        Ok(Some(tx))
    }

    async fn transaction_receipt(&self, _hash: H256) -> Result<Option<TransactionReceipt>> {
        let receipt = self.kakarot_client.transaction_receipt(_hash).await?;
        Ok(receipt)
    }

    async fn balance(&self, _address: Address, _block_number: Option<BlockId>) -> Result<U256> {
        let starknet_block_id =
            ethers_block_id_to_starknet_block_id(_block_number.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)))?;

        let balance = self.kakarot_client.balance(_address, starknet_block_id).await?;
        Ok(balance)
    }

    async fn storage_at(&self, _address: Address, _index: U256, _block_number: Option<BlockId>) -> Result<H256> {
        todo!()
    }

    async fn transaction_count(&self, _address: Address, _block_number: Option<BlockId>) -> Result<U256> {
        Ok(U256::from(3))
    }

    async fn get_code(&self, _address: Address, _block_number: Option<BlockId>) -> Result<Bytes> {
        let starknet_block_id = ethers_block_id_to_starknet_block_id(_block_number.unwrap())?;

        let code = self.kakarot_client.get_code(_address, starknet_block_id).await?;
        Ok(code)
    }

    async fn call(&self, _request: CallRequest, _block_number: Option<BlockId>) -> Result<Bytes> {
        // unwrap option or return jsonrpc error
        let to = _request.to.ok_or_else(|| {
            jsonrpsee::core::Error::Call(CallError::InvalidParams(anyhow::anyhow!(
                "CallRequest `to` field is None. Cannot process a Kakarot call",
            )))
        })?;

        let calldata = _request.data.ok_or_else(|| {
            jsonrpsee::core::Error::Call(CallError::InvalidParams(anyhow::anyhow!(
                "CallRequest `data` field is None. Cannot process a Kakarot call",
            )))
        })?;

        let block_id = _block_number.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let result = self.kakarot_client.call_view(to, Bytes::from(calldata.0), starknet_block_id).await?;

        Ok(result)
    }

    async fn create_access_list(
        &self,
        _request: CallRequest,
        _block_number: Option<BlockId>,
    ) -> Result<AccessListWithGasUsed> {
        todo!()
    }

    async fn estimate_gas(&self, _request: CallRequest, _block_number: Option<BlockId>) -> Result<U256> {
        Ok(U256::from(1_000_000_000_u64))
    }

    async fn gas_price(&self) -> Result<U256> {
        let gas_price = self.kakarot_client.base_fee_per_gas();
        Ok(gas_price)
    }

    async fn fee_history(
        &self,
        _block_count: U256,
        _newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory> {
        let fee_history = self.kakarot_client.fee_history(_block_count, _newest_block, _reward_percentiles).await?;

        Ok(fee_history)
    }

    async fn max_priority_fee_per_gas(&self) -> Result<U128> {
        let max_priority_fee = self.kakarot_client.max_priority_fee_per_gas();
        Ok(max_priority_fee)
    }

    async fn is_mining(&self) -> Result<bool> {
        Err(jsonrpsee::core::Error::Custom("Unsupported method: eth_mining. See available methods at https://github.com/sayajin-labs/kakarot-rpc/blob/main/docs/rpc_api_status.md".to_string()))
    }

    async fn hashrate(&self) -> Result<U256> {
        Err(jsonrpsee::core::Error::Custom("Unsupported method: eth_hashrate. See available methods at https://github.com/sayajin-labs/kakarot-rpc/blob/main/docs/rpc_api_status.md".to_string()))
    }

    async fn get_work(&self) -> Result<Work> {
        Err(jsonrpsee::core::Error::Custom("Unsupported method: eth_getWork. See available methods at https://github.com/sayajin-labs/kakarot-rpc/blob/main/docs/rpc_api_status.md".to_string()))
    }

    async fn submit_hashrate(&self, _hashrate: U256, _id: H256) -> Result<bool> {
        todo!()
    }

    async fn submit_work(&self, _nonce: H64, _pow_hash: H256, _mix_digest: H256) -> Result<bool> {
        todo!()
    }

    async fn send_transaction(&self, _request: TransactionRequest) -> Result<H256> {
        todo!()
    }

    async fn send_raw_transaction(&self, _bytes: Bytes) -> Result<H256> {
        let transaction_hash = self.kakarot_client.send_transaction(_bytes).await?;
        Ok(transaction_hash)
    }

    async fn sign(&self, _address: Address, _message: Bytes) -> Result<Bytes> {
        todo!()
    }

    async fn sign_transaction(&self, _transaction: CallRequest) -> Result<Bytes> {
        todo!()
    }

    async fn sign_typed_data(&self, _address: Address, _data: Value) -> Result<Bytes> {
        todo!()
    }

    async fn get_proof(
        &self,
        _address: Address,
        _keys: Vec<H256>,
        _block_number: Option<BlockId>,
    ) -> Result<EIP1186AccountProofResponse> {
        todo!()
    }
}

#[rpc(server, client)]
trait KakarotCustomApi {
    #[method(name = "kakarot_getTokenBalances")]
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances>;
}

#[async_trait]
impl KakarotCustomApiServer for KakarotEthRpc {
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances> {
        let token_balances = self.kakarot_client.token_balances(address, contract_addresses).await?;
        Ok(token_balances)
    }
}

impl KakarotEthRpc {
    #[must_use]
    pub fn new(kakarot_client: Box<dyn KakarotClient>) -> Self {
        Self { kakarot_client }
    }
}
