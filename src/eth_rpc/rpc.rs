use crate::{
    eth_provider::provider::EthereumProvider,
    eth_rpc::{
        api::{
            alchemy_api::AlchemyApiServer, debug_api::DebugApiServer, eth_api::EthApiServer,
            kakarot_api::KakarotApiServer, net_api::NetApiServer, trace_api::TraceApiServer,
            txpool_api::TxPoolApiServer, web3_api::Web3ApiServer,
        },
        servers::{
            alchemy_rpc::AlchemyRpc, debug_rpc::DebugRpc, eth_rpc::KakarotEthRpc, kakarot_rpc::KakarotRpc,
            net_rpc::NetRpc, trace_rpc::TraceRpc, txpool_rpc::TxpoolRpc, web3_rpc::Web3Rpc,
        },
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
pub struct KakarotRpcModuleBuilder<EP, SP>
where
    EP: EthereumProvider + Send + Sync,
    SP: Provider + Send + Sync,
{
    modules: HashMap<KakarotRpcModule, Methods>,
    _phantom: PhantomData<(EP, SP)>,
}

impl<EP, SP> KakarotRpcModuleBuilder<EP, SP>
where
    EP: EthereumProvider + Send + Sync + 'static,
    SP: Provider + Send + Sync + 'static,
{
    pub fn new(eth_provider: EP, starknet_provider: SP) -> Self {
        let eth_provider = Arc::new(eth_provider);
        let eth_rpc_module = KakarotEthRpc::new(eth_provider.clone()).into_rpc();
        let alchemy_rpc_module = AlchemyRpc::new(eth_provider.clone()).into_rpc();
        let web3_rpc_module = Web3Rpc::default().into_rpc();
        let net_rpc_module = NetRpc::new(eth_provider.clone()).into_rpc();
        let debug_rpc_module = DebugRpc::new(eth_provider.clone()).into_rpc();
        let trace_rpc_module = TraceRpc::new(eth_provider.clone()).into_rpc();
        let kakarot_rpc_module = KakarotRpc::new(eth_provider.clone(), starknet_provider).into_rpc();
        let txpool_rpc_module = TxpoolRpc::new(eth_provider).into_rpc();

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
