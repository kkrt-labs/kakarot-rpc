use super::KakarotEthApi;
use reth_rpc::EthApi;
use reth_rpc_eth_api::helpers::SpawnBlocking;
use std::future::Future;
use tokio::sync::{AcquireError, OwnedSemaphorePermit};

impl<Provider, Pool, Network, EvmConfig> SpawnBlocking for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Self: Clone + Send + Sync + 'static,
    EthApi<Provider, Pool, Network, EvmConfig>: SpawnBlocking,
{
    #[inline]
    fn io_task_spawner(&self) -> impl reth_tasks::TaskSpawner {
        self.0.task_spawner()
    }

    #[inline]
    fn tracing_task_pool(&self) -> &reth_tasks::pool::BlockingTaskPool {
        self.0.blocking_task_pool()
    }

    fn acquire_owned(&self) -> impl Future<Output = Result<OwnedSemaphorePermit, AcquireError>> + Send {
        self.0.acquire_owned()
    }

    fn acquire_many_owned(&self, n: u32) -> impl Future<Output = Result<OwnedSemaphorePermit, AcquireError>> + Send {
        self.0.acquire_many_owned(n)
    }
}
