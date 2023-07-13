use std::str::FromStr;

use lazy_static::lazy_static;
use reth_primitives::Address;
use starknet_crypto::FieldElement;

pub const PROXY_ACCOUNT_CLASS_HASH_HEX: &str = "0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5";
pub const ABDEL_STARKNET_ADDRESS_HEX: &str = "0xabde1";

pub const OTHER_PROXY_ACCOUNT_CLASS_HASH_HEX: &str =
    "0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef1";
pub const OTHER_ADDRESS_HEX: &str = "0x744ed080b42c8883a7e31cd11a14b7ae9ef27698b785486bb75cd116c8f1485";

pub const ACCOUNT_ADDRESS_HEX: &str = "0x044021e020d096bd375bddc0f8d122ecae520003ca4c2691cccaa9ad5b53eed7";
pub const ACCOUNT_PUBLIC_HEX: &str = "0x05f8d139ff7b7ad69bed4f71a775a3ccb5efaaeedd1cc3a63ff51a725f9b9738";

// Mock values
lazy_static! {
    /// Test value for Kakarot contract address.
    pub static ref KAKAROT_ADDRESS: FieldElement =
        FieldElement::from_hex_be("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",).unwrap();
    /// Test value for Abdel starknet address.
    pub static ref ABDEL_STARKNET_ADDRESS: FieldElement = FieldElement::from_hex_be(ABDEL_STARKNET_ADDRESS_HEX).unwrap();
    /// Test value for Abdel ethereum address.
    pub static ref ABDEL_ETHEREUM_ADDRESS: Address = Address::from_str("0x54b288676b749def5fc10eb17244fe2c87375de1").unwrap();
    /// Test value for proxy account class hash.
    pub static ref PROXY_ACCOUNT_CLASS_HASH: FieldElement =
        FieldElement::from_hex_be(PROXY_ACCOUNT_CLASS_HASH_HEX).unwrap();
}

// Testnet values
pub const INC_DATA: &str = "0x371303c0";
pub const KAKAROT_TESTNET_ADDRESS: &str = "0x01e98a4d6cadc1e3511d150ef2705b02fccb3fb6f15aba863503af58f4b217ea";
lazy_static! {
    pub static ref ACCOUNT_ADDRESS: FieldElement = FieldElement::from_hex_be(ACCOUNT_ADDRESS_HEX).unwrap();
    pub static ref COUNTER_ADDRESS: FieldElement =
        FieldElement::from_hex_be("0x03c12643f0e9f0b41de95a87e4f03f5fa69601930e9354a206a0b82a02119f2b").unwrap();
    pub static ref ACCOUNT_ADDRESS_EVM: Address =
        Address::from_str("0x54B288676B749DEF5FC10EB17244FE2C87375dE1").unwrap();
    pub static ref COUNTER_ADDRESS_EVM: Address =
        Address::from_str("0x2e11ed82f5ec165ab8ce3cc094f025fe7527f4d1").unwrap();
}
