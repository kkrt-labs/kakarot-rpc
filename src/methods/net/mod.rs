use jsonrpc_http_server::jsonrpc_core::{IoHandler, Params};

pub mod listening;
pub mod peer_count;
pub mod version;

pub fn init(io: &mut IoHandler) {
    io.add_method(&peer_count::METHOD.full_name(), |_params: Params| async {
        peer_count::execute(_params).await
    });

    io.add_method(&version::METHOD.full_name(), |_params: Params| async {
        version::execute(_params).await
    });

    io.add_method(&listening::METHOD.full_name(), |_params: Params| async {
        listening::execute(_params).await
    });
}
