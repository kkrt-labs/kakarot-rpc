use jsonrpc_http_server::jsonrpc_core::{IoHandler, Params};

pub mod client_version;
pub mod sha_3;

pub fn init(io: &mut IoHandler) {
    io.add_method(
        &client_version::METHOD.full_name(),
        |_params: Params| async { client_version::execute(_params).await },
    );

    io.add_method(&sha_3::METHOD.full_name(), |_params: Params| async {
        sha_3::execute(_params).await
    });
}
