use reth_revm::{inspector_handle_register, primitives::EnvWithHandlerCfg, Database, Evm};

#[derive(Debug, Clone)]
pub(super) struct EvmBuilder;

impl EvmBuilder {
    /// Returns new EVM with the given database, env and inspector. Similar to the implementation of [`reth_evm::ConfigureEvmEnv`]
    /// but only keeping the necessary API.
    pub(super) fn evm_with_env_and_inspector<'a, DB: Database + 'a, I: reth_revm::Inspector<DB>>(
        db: DB,
        env: EnvWithHandlerCfg,
        inspector: I,
    ) -> Evm<'a, I, DB> {
        let mut evm = reth_revm::EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .append_handler_register(inspector_handle_register)
            .build();
        evm.modify_spec_id(env.spec_id());
        evm.context.evm.env = env.env;
        evm
    }

    /// Returns new EVM with the given database and env. Similar to the implementation of [`reth_evm::ConfigureEvmEnv`]
    /// but only keeping the necessary API.
    pub(super) fn evm_with_env<'a, DB: Database + 'a>(db: DB, env: EnvWithHandlerCfg) -> Evm<'a, (), DB> {
        let mut evm = reth_revm::EvmBuilder::default().with_db(db).build();
        evm.modify_spec_id(env.spec_id());
        evm.context.evm.env = env.env;
        evm
    }
}
