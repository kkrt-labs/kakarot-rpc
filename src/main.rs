use clap::Parser;
use jsonrpc_http_server::{jsonrpc_core::IoHandler, ServerBuilder};

extern crate crypto;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    port: u16,
}

fn main() {
    let args = Args::parse();
    let io = IoHandler::default();

    let server = ServerBuilder::new(io)
        .threads(3)
        .start_http(
            &("127.0.0.1:".to_string() + &args.port.to_string())
                .parse()
                .unwrap(),
        )
        .unwrap();

    println!("Kakarot RPC Adapter running on port {} !", args.port);

    server.wait();
}
