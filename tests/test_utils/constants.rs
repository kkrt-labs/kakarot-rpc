use std::str::FromStr;

use lazy_static::lazy_static;
use reth_primitives::Address;
use starknet_crypto::FieldElement;

pub const ACCOUNT_ADDRESS_HEX: &str = "0x044021e020d096bd375bddc0f8d122ecae520003ca4c2691cccaa9ad5b53eed7";

// Testnet values
// TODO: Delete when simulateTransaction is merged in Madara
lazy_static! {
    pub static ref ACCOUNT_ADDRESS: FieldElement = FieldElement::from_hex_be(ACCOUNT_ADDRESS_HEX).unwrap();
    pub static ref ACCOUNT_ADDRESS_EVM: Address =
        Address::from_str("0x54B288676B749DEF5FC10EB17244FE2C87375dE1").unwrap();
    pub static ref COUNTER_ADDRESS_EVM: Address =
        Address::from_str("0x2e11ed82f5ec165ab8ce3cc094f025fe7527f4d1").unwrap();
}
