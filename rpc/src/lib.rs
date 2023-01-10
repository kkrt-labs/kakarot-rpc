//! Kakarot RPC module for Ethereum.
//! It is an adapter layer to interact with Kakarot ZK-EVM.
mod eth_rpc;

#[macro_use]
extern crate rocket;
use rocket::{Build, Rocket};
use rocket_okapi::{openapi, openapi_get_routes};

pub async fn build_rocket_server() -> Rocket<Build> {
    // Build Kakarot RPC
    let kakarot_rpc = eth_rpc::KakarotEthRpc::new().await;
    rocket::build()
        .manage(kakarot_rpc)
        .mount("/", openapi_get_routes![index,])
}

#[openapi]
#[get("/")]
pub fn index() -> &'static str {
    "Kamehameha!"
}
