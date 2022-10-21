#![allow(unused)]

use futures::{
    future::{self, Ready},
    prelude::*,
};

use tarpc::{
    client, context,
    server::{self, incoming::Incoming, Channel},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("👋 Hello, Kakarot!");
    Ok(())
}
