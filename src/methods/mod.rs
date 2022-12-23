use jsonrpc_http_server::jsonrpc_core::IoHandler;
pub mod net;
pub mod web3;

pub struct Method {
    pub prefix: &'static str,
    pub name: &'static str,
}

impl Method {
    pub fn full_name(self) -> String {
        let mut full_name = self.prefix.to_string();
        full_name.push_str("_");
        full_name.push_str(&self.name);
        return full_name;
    }
}

pub fn init(io: &mut IoHandler) {
    web3::init(io);
    net::init(io);
    ()
}
