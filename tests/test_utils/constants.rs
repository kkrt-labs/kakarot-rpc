use serde_json::Value;
use starknet_crypto::FieldElement;

lazy_static::lazy_static! {
    pub static ref KAKAROT_ADDRESS: FieldElement = {
        let deployments = include_str!("../../lib/kakarot/deployments/katana/deployments.json");

    let object: Value = serde_json::from_str(deployments).unwrap();

    object.get("kakarot").unwrap().get("address").unwrap().as_str().unwrap().parse().unwrap()
    };
}
