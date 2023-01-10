//! Kakarot RPC module for Ethereum.
//! It is an adapter layer to interact with Kakarot ZK-EVM.
mod eth_rpc;

#[macro_use]
extern crate rocket;
use rocket::{Build, Rocket};
use rocket_okapi::{openapi, openapi_get_routes};

pub async fn build_rocket_server() -> Rocket<Build> {
    rocket::build().mount("/", openapi_get_routes![index,])
}

#[openapi]
#[get("/")]
pub fn index() -> &'static str {
    "Kamehameha!"
}
