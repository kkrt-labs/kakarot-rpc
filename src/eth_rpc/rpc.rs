use crate::{
    client::EthClient,
    eth_rpc::{
        api::{
            alchemy_api::AlchemyApiServer, debug_api::DebugApiServer, eth_api::EthApiServer,
            kakarot_api::KakarotApiServer, net_api::NetApiServer, trace_api::TraceApiServer,
            txpool_api::TxPoolApiServer, web3_api::Web3ApiServer,
        },
        servers::{
            alchemy_rpc::AlchemyRpc, debug_rpc::DebugRpc, eth_rpc::EthRpc, kakarot_rpc::KakarotRpc, net_rpc::NetRpc,
            trace_rpc::TraceRpc, txpool_rpc::TxpoolRpc, web3_rpc::Web3Rpc,
        },
    },
    providers::{
        alchemy_provider::AlchemyDataProvider, debug_provider::DebugDataProvider, pool_provider::PoolDataProvider,
    },
};
use jsonrpsee::{server::RegisterMethodError, Methods, RpcModule};
use starknet::providers::Provider;
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

/// Represents RPC modules that are supported by reth
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum KakarotRpcModule {
    Eth,
    Alchemy,
    Web3,
    Net,
    Debug,
    Trace,
    Txpool,
    KakarotRpc,
}

#[derive(Debug)]
pub struct KakarotRpcModuleBuilder<SP> {
    modules: HashMap<KakarotRpcModule, Methods>,
    _phantom: PhantomData<SP>,
}

impl<SP> KakarotRpcModuleBuilder<SP>
where
    SP: Provider + Clone + Send + Sync + 'static,
{
    pub fn new(eth_client: EthClient<SP>) -> Self {
        let eth_provider = eth_client.eth_provider().clone();
        let starknet_provider = eth_provider.starknet_provider_inner().clone();

        let alchemy_provider = Arc::new(AlchemyDataProvider::new(eth_provider.clone()));
        let pool_provider = Arc::new(PoolDataProvider::new(eth_provider.clone()));
        let debug_provider = Arc::new(DebugDataProvider::new(eth_provider.clone()));

        let eth_rpc_module = EthRpc::new(eth_client).into_rpc();
        let alchemy_rpc_module = AlchemyRpc::new(alchemy_provider).into_rpc();
        let web3_rpc_module = Web3Rpc::default().into_rpc();
        let net_rpc_module = NetRpc::new(eth_provider.clone()).into_rpc();
        let debug_rpc_module = DebugRpc::new(debug_provider).into_rpc();
        let trace_rpc_module = TraceRpc::new(eth_provider.clone()).into_rpc();
        let kakarot_rpc_module = KakarotRpc::new(eth_provider, starknet_provider).into_rpc();
        let txpool_rpc_module = TxpoolRpc::new(pool_provider).into_rpc();

        let mut modules = HashMap::new();

        modules.insert(KakarotRpcModule::Eth, eth_rpc_module.into());
        modules.insert(KakarotRpcModule::Alchemy, alchemy_rpc_module.into());
        modules.insert(KakarotRpcModule::Web3, web3_rpc_module.into());
        modules.insert(KakarotRpcModule::Net, net_rpc_module.into());
        modules.insert(KakarotRpcModule::Debug, debug_rpc_module.into());
        modules.insert(KakarotRpcModule::Trace, trace_rpc_module.into());
        modules.insert(KakarotRpcModule::Txpool, txpool_rpc_module.into());
        modules.insert(KakarotRpcModule::KakarotRpc, kakarot_rpc_module.into());

        Self { modules, _phantom: PhantomData }
    }

    pub fn rpc_module(&self) -> Result<RpcModule<()>, RegisterMethodError> {
        let mut rpc_module = RpcModule::new(());

        for methods in self.modules.values().cloned() {
            rpc_module.merge(methods)?;
        }

        Ok(rpc_module)
    }
}
