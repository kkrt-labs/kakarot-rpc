use jsonrpc_http_server::jsonrpc_core::{IoHandler, Params};

pub mod chain_id;

pub fn init(io: &mut IoHandler) {
    io.add_method(&chain_id::METHOD.full_name(), |_params: Params| async {
        chain_id::execute(_params).await
    });
}
