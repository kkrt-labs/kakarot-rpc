use kakarot_rpc::build_rocket_server;
use rocket::{Build, Rocket};
#[macro_use]
extern crate rocket;

#[launch]
async fn rocket() -> Rocket<Build> {
    env_logger::init();

    info!("starting Kakarot RPC...");
    build_rocket_server().await
}
