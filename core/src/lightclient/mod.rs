use eyre::Result;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};
use url::Url;

pub struct StarknetClient {
    client: JsonRpcClient<HttpTransport>,
}

impl StarknetClient {
    pub fn new(starknet_rpc: &str) -> Result<Self> {
        let url = Url::parse(starknet_rpc)?;
        Ok(Self {
            client: JsonRpcClient::new(HttpTransport::new(url)),
        })
    }

    /// Get the number of transactions in a block given a block id.
    /// The number of transactions in a block.
    ///
    /// # Arguments
    ///
    ///
    ///
    /// # Returns
    ///
    ///  * `block_number(u64)` - The block number.
    ///
    /// `Ok(ContractClass)` if the operation was successful.
    /// `Err(eyre::Report)` if the operation failed.
    pub async fn block_number(&self) -> Result<u64> {
        self.client.block_number().await.map_err(|e| eyre::eyre!(e))
    }

        //TODO: uncomment when changing to RETH trait
    // /// Create a new KakarotEthRpc instance.
    // fn protocol_version(&self) -> Result<U64> {
    //     todo!()
    // }

    // fn syncing(&self) -> Result<SyncStatus> {
    //     todo!()
    // }

    // async fn author(&self) -> Result<Address> {
    //     todo!()
    // }

    // async fn accounts(&self) -> Result<Vec<Address>> {
    //     todo!()
    // }

    // fn block_number(&self) -> Result<U256> {
    //     todo!()
    // }

    // async fn chain_id(&self) -> Result<Option<U64>> {
    //     todo!()
    // }

    // async fn block_by_hash(&self, _hash: H256, _full: bool) -> Result<Option<RichBlock>> {
    //     todo!()
    // }

    // async fn block_by_number(
    //     &self,
    //     _number: BlockNumber,
    //     _full: bool,
    // ) -> Result<Option<RichBlock>> {
    //     todo!()
    // }

    // async fn block_transaction_count_by_hash(&self, _hash: H256) -> Result<Option<U256>> {
    //     todo!()
    // }

    // async fn block_transaction_count_by_number(
    //     &self,
    //     _number: BlockNumber,
    // ) -> Result<Option<U256>> {
    //     todo!()
    // }

    // async fn block_uncles_count_by_hash(&self, _hash: H256) -> Result<U256> {
    //     todo!()
    // }

    // async fn block_uncles_count_by_number(&self, _number: BlockNumber) -> Result<U256> {
    //     todo!()
    // }

    // async fn uncle_by_block_hash_and_index(
    //     &self,
    //     _hash: H256,
    //     _index: Index,
    // ) -> Result<Option<RichBlock>> {
    //     todo!()
    // }

    // async fn uncle_by_block_number_and_index(
    //     &self,
    //     _number: BlockNumber,
    //     _index: Index,
    // ) -> Result<Option<RichBlock>> {
    //     todo!()
    // }

    // async fn transaction_by_hash(
    //     &self,
    //     _hash: H256,
    // ) -> Result<Option<reth_rpc_types::Transaction>> {
    //     todo!()
    // }

    // async fn transaction_by_block_hash_and_index(
    //     &self,
    //     _hash: H256,
    //     _index: Index,
    // ) -> Result<Option<reth_rpc_types::Transaction>> {
    //     todo!()
    // }

    // async fn transaction_by_block_number_and_index(
    //     &self,
    //     _number: BlockNumber,
    //     _index: Index,
    // ) -> Result<Option<reth_rpc_types::Transaction>> {
    //     todo!()
    // }

    // async fn transaction_receipt(&self, _hash: H256) -> Result<Option<TransactionReceipt>> {
    //     todo!()
    // }

    // async fn balance(&self, _address: Address, _block_number: Option<BlockId>) -> Result<U256> {
    //     todo!()
    // }

    // async fn storage_at(
    //     &self,
    //     _address: Address,
    //     _index: U256,
    //     _block_number: Option<BlockId>,
    // ) -> Result<H256> {
    //     todo!()
    // }

    // async fn transaction_count(
    //     &self,
    //     _address: Address,
    //     _block_number: Option<BlockId>,
    // ) -> Result<U256> {
    //     todo!()
    // }

    // async fn get_code(&self, _address: Address, _block_number: Option<BlockId>) -> Result<Bytes> {
    //     todo!()
    // }

    // async fn call(&self, _request: CallRequest, _block_number: Option<BlockId>) -> Result<Bytes> {
    //     todo!()
    // }

    // async fn create_access_list(
    //     &self,
    //     _request: CallRequest,
    //     _block_number: Option<BlockId>,
    // ) -> Result<AccessListWithGasUsed> {
    //     todo!()
    // }

    // async fn estimate_gas(
    //     &self,
    //     _request: CallRequest,
    //     _block_number: Option<BlockId>,
    // ) -> Result<U256> {
    //     todo!()
    // }

    // async fn gas_price(&self) -> Result<U256> {
    //     todo!()
    // }

    // async fn fee_history(
    //     &self,
    //     _block_count: U256,
    //     _newest_block: BlockNumber,
    //     _reward_percentiles: Option<Vec<f64>>,
    // ) -> Result<FeeHistory> {
    //     todo!()
    // }

    // async fn max_priority_fee_per_gas(&self) -> Result<U256> {
    //     todo!()
    // }

    // async fn is_mining(&self) -> Result<bool> {
    //     todo!()
    // }

    // async fn hashrate(&self) -> Result<U256> {
    //     todo!()
    // }

    // async fn get_work(&self) -> Result<Work> {
    //     todo!()
    // }

    // async fn submit_hashrate(&self, _hashrate: U256, _id: H256) -> Result<bool> {
    //     todo!()
    // }

    // async fn submit_work(&self, _nonce: H64, _pow_hash: H256, _mix_digest: H256) -> Result<bool> {
    //     todo!()
    // }

    // async fn send_transaction(&self, _request: TransactionRequest) -> Result<H256> {
    //     todo!()
    // }

    // async fn send_raw_transaction(&self, _bytes: Bytes) -> Result<H256> {
    //     todo!()
    // }
}

