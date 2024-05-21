use std::sync::Arc;

use alphanet_instructions::{context::InstructionsContext, eip3074};
use reth_revm::{
    handler::register::HandleRegisterBox, inspector_handle_register, primitives::EnvWithHandlerCfg, Database, Evm,
};

#[derive(Debug, Clone)]
pub(super) struct EvmBuilder;

impl EvmBuilder {
    /// Returns new EVM with the given database, env and inspector. Similar to the implementation of [reth_evm::ConfigureEvmEnv]
    /// but only keeping the necessary API.
    pub(super) fn evm_with_env_and_inspector<'a, DB: Database + 'a, I: reth_revm::Inspector<DB>>(
        db: DB,
        env: EnvWithHandlerCfg,
        inspector: I,
    ) -> Evm<'a, I, DB> {
        let mut evm = reth_revm::EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .append_handler_register_box(eip3074_handle_register())
            .append_handler_register(inspector_handle_register)
            .build();
        evm.modify_spec_id(env.spec_id());
        evm.context.evm.env = env.env;
        evm
    }

    /// Returns new EVM with the given database and env. Similar to the implementation of [reth_evm::ConfigureEvmEnv]
    /// but only keeping the necessary API.
    pub(super) fn evm_with_env<'a, DB: Database + 'a>(db: DB, env: EnvWithHandlerCfg) -> Evm<'a, (), DB> {
        let mut evm =
            reth_revm::EvmBuilder::default().with_db(db).append_handler_register_box(eip3074_handle_register()).build();
        evm.modify_spec_id(env.spec_id());
        evm.context.evm.env = env.env;
        evm
    }
}

fn eip3074_handle_register<EXT, DB: Database>() -> HandleRegisterBox<EXT, DB> {
    let instructions_context = InstructionsContext::default();
    let to_capture_instructions = instructions_context.clone();

    Box::new(move |handler| {
        if let Some(ref mut table) = handler.instruction_table {
            for boxed_instruction_with_opcode in eip3074::boxed_instructions(to_capture_instructions.clone()) {
                table.insert_boxed(
                    boxed_instruction_with_opcode.opcode,
                    boxed_instruction_with_opcode.boxed_instruction,
                );
            }
        }
        let post_execution_context = instructions_context.clone();
        handler.post_execution.end = Arc::new(move |_, outcome: _| {
            // at the end of the transaction execution we clear the instructions
            post_execution_context.clear();
            outcome
        });
    })
}
