use std::str::FromStr;

use lazy_static::lazy_static;
use reth_primitives::Address;
use starknet_crypto::FieldElement;

pub const PROXY_ACCOUNT_CLASS_HASH_HEX: &str = "0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5";
pub const ABDEL_STARKNET_ADDRESS_HEX: &str = "0xabde1";

pub const OTHER_PROXY_ACCOUNT_CLASS_HASH_HEX: &str =
    "0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef1";
pub const OTHER_ADDRESS_HEX: &str = "0x744ed080b42c8883a7e31cd11a14b7ae9ef27698b785486bb75cd116c8f1485";

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
