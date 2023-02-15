use jsonrpsee::core::{async_trait, RpcResult as Result};
use jsonrpsee::proc_macros::rpc;

use jsonrpsee::types::error::CallError;
use kakarot_rpc_core::client::{constants::selectors::CHAIN_ID, types::RichBlock, KakarotClient};
use kakarot_rpc_core::helpers::{ethers_block_id_to_starknet_block_id, raw_calldata};
use reth_primitives::{
    rpc::{transaction::eip2930::AccessListWithGasUsed, BlockId, BlockNumber, Bytes, H256},
    Address, Bytes as PrimitiveBytes, TransactionSigned, H64, U256, U64,
};
use reth_rlp::Decodable;
use reth_rpc_types::{
    CallRequest, EIP1186AccountProofResponse, FeeHistory, Index, SyncStatus, TransactionReceipt,
    TransactionRequest, Work,
};
use serde_json::Value;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::models::{BlockId as StarknetBlockId, BlockTag};

use kakarot_rpc_core::client::types::Transaction as EtherTransaction;

/// The RPC module for the Ethereum protocol required by Kakarot.
///
///
pub struct KakarotEthRpc {
    pub kakarot_client: Box<dyn KakarotClient>,
}

#[rpc(server, client)]
trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> Result<U64>;

    /// Returns the protocol version encoded as a string.
    #[method(name = "net_version")]
    fn protocol_version(&self) -> Result<U64>;

    /// Returns an object with data about the sync status or false.
    #[method(name = "eth_syncing")]
    async fn syncing(&self) -> Result<SyncStatus>;

    /// Returns the client coinbase address.
    #[method(name = "eth_coinbase")]
    async fn author(&self) -> Result<Address>;

    /// Returns a list of addresses owned by client.
    #[method(name = "eth_accounts")]
    async fn accounts(&self) -> Result<Vec<Address>>;

    /// Returns the chain ID of the current network.
    #[method(name = "eth_chainId")]
    async fn chain_id(&self) -> Result<Option<U64>>;

    /// Returns information about a block by hash.
    #[method(name = "eth_getBlockByHash")]
    async fn block_by_hash(&self, hash: H256, full: bool) -> Result<Option<RichBlock>>;

    /// Returns information about a block by number.
    #[method(name = "eth_getBlockByNumber")]
    async fn block_by_number(&self, number: BlockNumber, full: bool) -> Result<Option<RichBlock>>;

    /// Returns the number of transactions in a block from a block matching the given block hash.
    #[method(name = "eth_getBlockTransactionCountByHash")]
    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>>;

    /// Returns the number of transactions in a block matching the given block number.
    #[method(name = "eth_getBlockTransactionCountByNumber")]
    async fn block_transaction_count_by_number(&self, number: BlockNumber) -> Result<Option<U256>>;

    /// Returns the number of uncles in a block from a block matching the given block hash.
    #[method(name = "eth_getUncleCountByBlockHash")]
    async fn block_uncles_count_by_hash(&self, hash: H256) -> Result<U256>;

    /// Returns the number of uncles in a block with given block number.
    #[method(name = "eth_getUncleCountByBlockNumber")]
    async fn block_uncles_count_by_number(&self, number: BlockNumber) -> Result<U256>;

    /// Returns an uncle block of the given block and index.
    #[method(name = "eth_getUncleByBlockHashAndIndex")]
    async fn uncle_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> Result<Option<RichBlock>>;

    /// Returns an uncle block of the given block and index.
    #[method(name = "eth_getUncleByBlockNumberAndIndex")]
    async fn uncle_by_block_number_and_index(
        &self,
        number: BlockNumber,
        index: Index,
    ) -> Result<Option<RichBlock>>;

    /// Returns the information about a transaction requested by transaction hash.
    #[method(name = "eth_getTransactionByHash")]
    async fn transaction_by_hash(&self, hash: H256) -> Result<Option<EtherTransaction>>;

    /// Returns information about a transaction by block hash and transaction index position.
    #[method(name = "eth_getTransactionByBlockHashAndIndex")]
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> Result<Option<EtherTransaction>>;

    /// Returns information about a transaction by block number and transaction index position.
    #[method(name = "eth_getTransactionByBlockNumberAndIndex")]
    async fn transaction_by_block_number_and_index(
        &self,
        number: BlockNumber,
        index: Index,
    ) -> Result<Option<EtherTransaction>>;

    /// Returns the receipt of a transaction by transaction hash.
    #[method(name = "eth_getTransactionReceipt")]
    async fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>>;

    /// Returns the balance of the account of given address.
    #[method(name = "eth_getBalance")]
    async fn balance(&self, address: Address, block_number: Option<BlockId>) -> Result<U256>;

    /// Returns the value from a storage position at a given address
    #[method(name = "eth_getStorageAt")]
    async fn storage_at(
        &self,
        address: Address,
        index: U256,
        block_number: Option<BlockId>,
    ) -> Result<H256>;

    /// Returns the number of transactions sent from an address at given block number.
    #[method(name = "eth_getTransactionCount")]
    async fn transaction_count(
        &self,
        address: Address,
        block_number: Option<BlockId>,
    ) -> Result<U256>;

    /// Returns code at a given address at given block number.
    #[method(name = "eth_getCode")]
    async fn get_code(&self, address: Address, block_number: Option<BlockId>) -> Result<Bytes>;

    /// Executes a new message call immediately without creating a transaction on the block chain.
    #[method(name = "eth_call")]
    async fn call(&self, request: CallRequest, block_number: Option<BlockId>) -> Result<Bytes>;

    /// Generates an access list for a transaction.
    ///
    /// This method creates an [EIP2930](https://eips.ethereum.org/EIPS/eip-2930) type accessList based on a given Transaction.
    ///
    /// An access list contains all storage slots and addresses touched by the transaction, except
    /// for the sender account and the chain's precompiles.
    ///
    /// It returns list of addresses and storage keys used by the transaction, plus the gas
    /// consumed when the access list is added. That is, it gives you the list of addresses and
    /// storage keys that will be used by that transaction, plus the gas consumed if the access
    /// list is included. Like eth_estimateGas, this is an estimation; the list could change
    /// when the transaction is actually mined. Adding an accessList to your transaction does
    /// not necessary result in lower gas usage compared to a transaction without an access
    /// list.
    #[method(name = "eth_createAccessList")]
    async fn create_access_list(
        &self,
        request: CallRequest,
        block_number: Option<BlockId>,
    ) -> Result<AccessListWithGasUsed>;

    /// Generates and returns an estimate of how much gas is necessary to allow the transaction to
    /// complete.
    #[method(name = "eth_estimateGas")]
    async fn estimate_gas(
        &self,
        request: CallRequest,
        block_number: Option<BlockId>,
    ) -> Result<U256>;

    /// Returns the current price per gas in wei.
    #[method(name = "eth_gasPrice")]
    async fn gas_price(&self) -> Result<U256>;

    /// Returns the Transaction fee history
    ///
    /// Introduced in EIP-1159 for getting information on the appropriate priority fee to use.
    ///
    /// Returns transaction base fee per gas and effective priority fee per gas for the
    /// requested/supported block range. The returned Fee history for the returned block range
    /// can be a subsection of the requested range if not all blocks are available.
    #[method(name = "eth_feeHistory")]
    async fn fee_history(
        &self,
        block_count: U256,
        newest_block: BlockNumber,
        reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory>;

    /// Returns the current maxPriorityFeePerGas per gas in wei.
    #[method(name = "eth_maxPriorityFeePerGas")]
    async fn max_priority_fee_per_gas(&self) -> Result<U256>;

    /// Returns whether the client is actively mining new blocks.
    #[method(name = "eth_mining")]
    async fn is_mining(&self) -> Result<bool>;

    /// Returns the number of hashes per second that the node is mining with.
    #[method(name = "eth_hashrate")]
    async fn hashrate(&self) -> Result<U256>;

    /// Returns the hash of the current block, the seedHash, and the boundary condition to be met
    /// (“target”)
    #[method(name = "eth_getWork")]
    async fn get_work(&self) -> Result<Work>;

    /// Used for submitting mining hashrate.
    #[method(name = "eth_submitHashrate")]
    async fn submit_hashrate(&self, hashrate: U256, id: H256) -> Result<bool>;

    /// Used for submitting a proof-of-work solution.
    #[method(name = "eth_submitWork")]
    async fn submit_work(&self, nonce: H64, pow_hash: H256, mix_digest: H256) -> Result<bool>;

    /// Sends transaction; will block waiting for signer to return the
    /// transaction hash.
    #[method(name = "eth_sendTransaction")]
    async fn send_transaction(&self, request: TransactionRequest) -> Result<H256>;

    /// Sends signed transaction, returning its hash.
    #[method(name = "eth_sendRawTransaction")]
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256>;

    /// Returns an Ethereum specific signature with: sign(keccak256("\x19Ethereum Signed Message:\n"
    /// + len(message) + message))).
    #[method(name = "eth_sign")]
    async fn sign(&self, address: Address, message: Bytes) -> Result<Bytes>;

    /// Signs a transaction that can be submitted to the network at a later time using with
    /// `eth_sendRawTransaction.`
    #[method(name = "eth_signTransaction")]
    async fn sign_transaction(&self, transaction: CallRequest) -> Result<Bytes>;

    /// Signs data via [EIP-712](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-712.md).
    #[method(name = "eth_signTypedData")]
    async fn sign_typed_data(&self, address: Address, data: serde_json::Value) -> Result<Bytes>;

    /// Returns the account and storage values of the specified account including the Merkle-proof.
    /// This call can be used to verify that the data you are pulling from is not tampered with.
    #[method(name = "eth_getProof")]
    async fn get_proof(
        &self,
        address: Address,
        keys: Vec<H256>,
        block_number: Option<BlockId>,
    ) -> Result<EIP1186AccountProofResponse>;
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
        let block_id = BlockId::Hash(_hash);
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let block = self
            .kakarot_client
            .get_eth_block_from_starknet_block(starknet_block_id, _full)
            .await?;
        Ok(Some(block))
    }

    async fn block_by_number(
        &self,
        _number: BlockNumber,
        _full: bool,
    ) -> Result<Option<RichBlock>> {
        let block_id = BlockId::Number(_number);
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let block = self
            .kakarot_client
            .get_eth_block_from_starknet_block(starknet_block_id, _full)
            .await?;
        Ok(Some(block))
    }

    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
        let count = self
            .kakarot_client
            .block_transaction_count_by_hash(hash)
            .await?;
        Ok(count)
    }

    async fn block_transaction_count_by_number(
        &self,
        _number: BlockNumber,
    ) -> Result<Option<U256>> {
        let transaction_count = self
            .kakarot_client
            .block_transaction_count_by_number(_number)
            .await?;
        match transaction_count {
            Some(transaction_count) => Ok(Some(transaction_count)),
            None => Ok(None),
        }
    }

    async fn block_uncles_count_by_hash(&self, _hash: H256) -> Result<U256> {
        todo!()
    }

    async fn block_uncles_count_by_number(&self, _number: BlockNumber) -> Result<U256> {
        todo!()
    }

    async fn uncle_by_block_hash_and_index(
        &self,
        _hash: H256,
        _index: Index,
    ) -> Result<Option<RichBlock>> {
        todo!()
    }

    async fn uncle_by_block_number_and_index(
        &self,
        _number: BlockNumber,
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
        let block_id = BlockId::Hash(_hash);
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let tx = self
            .kakarot_client
            .transaction_by_block_id_and_index(starknet_block_id, _index)
            .await?;
        Ok(Some(tx))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        _number: BlockNumber,
        _index: Index,
    ) -> Result<Option<EtherTransaction>> {
        let block_id = BlockId::Number(_number);
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let tx = self
            .kakarot_client
            .transaction_by_block_id_and_index(starknet_block_id, _index)
            .await?;
        Ok(Some(tx))
    }

    async fn transaction_receipt(&self, _hash: H256) -> Result<Option<TransactionReceipt>> {
        let receipt = self.kakarot_client.get_transaction_receipt(_hash).await?;
        Ok(receipt)
    }

    async fn balance(&self, _address: Address, _block_number: Option<BlockId>) -> Result<U256> {
        let starknet_block_id = ethers_block_id_to_starknet_block_id(
            _block_number.unwrap_or(BlockId::Number(BlockNumber::Latest)),
        )?;

        let balance = self
            .kakarot_client
            .balance(_address, starknet_block_id)
            .await?;
        Ok(balance)
    }

    async fn storage_at(
        &self,
        _address: Address,
        _index: U256,
        _block_number: Option<BlockId>,
    ) -> Result<H256> {
        todo!()
    }

    async fn transaction_count(
        &self,
        _address: Address,
        _block_number: Option<BlockId>,
    ) -> Result<U256> {
        Ok(U256::from(3))
    }

    async fn get_code(&self, _address: Address, _block_number: Option<BlockId>) -> Result<Bytes> {
        let starknet_block_id = ethers_block_id_to_starknet_block_id(_block_number.unwrap())?;

        let code = self
            .kakarot_client
            .get_code(_address, starknet_block_id)
            .await?;
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

        let block_id = _block_number.unwrap_or(BlockId::Number(BlockNumber::Latest));
        let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id)?;
        let result = self
            .kakarot_client
            .call_view(to, Bytes::from(calldata.0), starknet_block_id)
            .await?;

        Ok(result)
    }

    async fn create_access_list(
        &self,
        _request: CallRequest,
        _block_number: Option<BlockId>,
    ) -> Result<AccessListWithGasUsed> {
        todo!()
    }

    async fn estimate_gas(
        &self,
        _request: CallRequest,
        _block_number: Option<BlockId>,
    ) -> Result<U256> {
        Ok(U256::ZERO)
    }

    async fn gas_price(&self) -> Result<U256> {
        //TODO: Fetch correct gas price from Starknet / AA
        Ok(U256::from(100))
    }

    async fn fee_history(
        &self,
        _block_count: U256,
        _newest_block: BlockNumber,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory> {
        let base_fee_per_gas: Vec<U256> = vec![U256::from(16), U256::from(16), U256::from(16)];

        let gas_used_ratio: Vec<f64> = vec![];
        let newest_block = _newest_block.as_number().unwrap().as_u64();
        let oldest_block: U256 = U256::from(newest_block) - _block_count;

        let reward: Option<Vec<Vec<U256>>> = None;
        Ok(FeeHistory {
            base_fee_per_gas,
            gas_used_ratio,
            oldest_block,
            reward,
        })
    }

    async fn max_priority_fee_per_gas(&self) -> Result<U256> {
        Ok(U256::from(100))
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
        let mut data = _bytes.as_ref();

        if data.is_empty() {
            return Err(jsonrpsee::core::Error::Call(CallError::InvalidParams(
                anyhow::anyhow!("Raw transaction data is empty. Cannot process a Kakarot call",),
            )));
        };

        let transaction = TransactionSigned::decode(&mut data).map_err(|_| {
            jsonrpsee::core::Error::Call(CallError::InvalidParams(anyhow::anyhow!(
                "Failed to decode raw transaction data. Cannot process a Kakarot call",
            )))
        })?;

        println!("Transaction: {transaction:?}");

        let evm_address = transaction.recover_signer().ok_or_else(|| {
            jsonrpsee::core::Error::Call(CallError::InvalidParams(anyhow::anyhow!(
                "Failed to recover signer from raw transaction data. Cannot process a Kakarot call",
            )))
        })?;

        let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);

        let starknet_address = self
            .kakarot_client
            .compute_starknet_address(evm_address, &starknet_block_id)
            .await
            .map_err(|_| {
                jsonrpsee::core::Error::Call(CallError::InvalidParams(anyhow::anyhow!(
                    "Failed to get starknet address from evm address. Cannot process a Kakarot call",
                )))
            })?;

        // TODO: Get nonce from Starknet
        let nonce = FieldElement::from(transaction.nonce());
        // TODO: Get gas price from Starknet
        let max_fee = FieldElement::from(1_000_000_000_000_u64);
        // TODO: Provide signature
        let signature = vec![];

        let calldata = raw_calldata(PrimitiveBytes::from(_bytes.0)).map_err(|_| {
            jsonrpsee::core::Error::Call(CallError::InvalidParams(anyhow::anyhow!(
                "Failed to get calldata from raw transaction data. Cannot process a Kakarot call",
            )))
        })?;

        let starknet_transaction_hash = self
            .kakarot_client
            .submit_starknet_transaction(max_fee, signature, nonce, starknet_address, calldata)
            .await?;

        Ok(starknet_transaction_hash)
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

impl KakarotEthRpc {
    pub fn new(kakarot_client: Box<dyn KakarotClient>) -> Self {
        Self { kakarot_client }
    }
}
