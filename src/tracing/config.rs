use reth_revm::{inspector_handle_register, primitives::EnvWithHandlerCfg, Database, Evm, EvmBuilder};

#[derive(Debug, Clone)]
pub(super) struct KakarotEvmConfig;

impl KakarotEvmConfig {
    /// Returns new EVM with the given database and env. We would prefer to implement [reth_evm::ConfigureEvmEnv]
    /// but need this commit to be merged in order to add `append_handler_register(inspector_handle_register)`
    /// otherwise the inspector will not be registered.
    /// https://github.com/paradigmxyz/reth/pull/7470
    pub(super) fn evm_with_env_and_inspector<'a, DB: Database + 'a, I: reth_revm::Inspector<DB>>(
        &self,
        db: DB,
        env: EnvWithHandlerCfg,
        inspector: I,
    ) -> Evm<'a, I, DB> {
        let mut evm = EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .append_handler_register(inspector_handle_register)
            .build();
        evm.modify_spec_id(env.spec_id());
        evm.context.evm.env = env.env;
        evm
    }
}
