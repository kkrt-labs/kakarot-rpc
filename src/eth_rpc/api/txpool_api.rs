use crate::providers::eth_provider::database::types::transaction::ExtendedTransaction;
use alloy_primitives::Address;
use alloy_rpc_types_txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolStatus};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

/// Txpool API
#[rpc(server, namespace = "txpool")]
#[async_trait]
pub trait TxPoolApi {
    /// Returns the number of transactions currently pending for inclusion in the next block(s), as
    /// well as the ones that are being scheduled for future execution only.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_status) for more details
    #[method(name = "status")]
    async fn txpool_status(&self) -> RpcResult<TxpoolStatus>;

    /// Returns a summary of all the transactions currently pending for inclusion in the next
    /// block(s), as well as the ones that are being scheduled for future execution only.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_inspect) for more details
    #[method(name = "inspect")]
    async fn txpool_inspect(&self) -> RpcResult<TxpoolInspect>;

    /// Retrieves the transactions contained within the txpool, returning pending
    /// transactions of this address, grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_contentFrom) for more details
    #[method(name = "contentFrom")]
    async fn txpool_content_from(&self, from: Address) -> RpcResult<TxpoolContentFrom<ExtendedTransaction>>;

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s), grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_content) for more details
    #[method(name = "content")]
    async fn txpool_content(&self) -> RpcResult<TxpoolContent<ExtendedTransaction>>;
}
