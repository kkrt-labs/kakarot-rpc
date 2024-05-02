use std::sync::Arc;

use alphanet_instructions::{context::InstructionsContext, eip3074};
use reth_revm::{inspector_handle_register, primitives::EnvWithHandlerCfg, Database, Evm, EvmBuilder};

#[derive(Debug, Clone)]
pub(super) struct KakarotEvmConfig;

impl KakarotEvmConfig {
    /// Returns new EVM with the given database and env. Similar to the implementation of [reth_evm::ConfigureEvmEnv]
    /// but only keeping the necessary API.
    pub(super) fn evm_with_env_and_inspector<'a, DB: Database + 'a, I: reth_revm::Inspector<DB>>(
        &self,
        db: DB,
        env: EnvWithHandlerCfg,
        inspector: I,
    ) -> Evm<'a, I, DB> {
        let instructions_context = InstructionsContext::default();
        let to_capture_instructions = instructions_context.clone();

        let mut evm = EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .append_handler_register_box(Box::new(move |handler| {
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
            }))
            .append_handler_register(inspector_handle_register)
            .build();
        evm.modify_spec_id(env.spec_id());
        evm.context.evm.env = env.env;
        evm
    }
}
