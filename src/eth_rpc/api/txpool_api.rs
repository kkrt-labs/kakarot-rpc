use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use reth_primitives::Address;
use reth_rpc_types::txpool::{TxpoolContent, TxpoolContentFrom};

/// Txpool API
#[rpc(server, namespace = "txpool")]
#[async_trait]
pub trait TxPoolApi {
    /// Retrieves the transactions contained within the txpool, returning pending
    /// transactions of this address, grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_contentFrom) for more details
    #[method(name = "contentFrom")]
    async fn txpool_content_from(&self, from: Address) -> RpcResult<TxpoolContentFrom>;

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s), grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_content) for more details
    #[method(name = "content")]
    async fn txpool_content(&self) -> RpcResult<TxpoolContent>;
}
