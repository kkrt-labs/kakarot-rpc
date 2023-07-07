use std::str::FromStr;

use lazy_static::lazy_static;
use reth_primitives::Address;
use starknet_crypto::FieldElement;

lazy_static! {
    /// Test value for Kakarot contract address.
    pub static ref KAKAROT_ADDRESS: FieldElement =
        FieldElement::from_hex_be("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",).unwrap();
    /// Test value for Abdel starknet address.
    pub static ref ABDEL_STARKNET_ADDRESS: FieldElement = FieldElement::from_hex_be("0x0abde1").unwrap();
    /// Test value for Abdel ethereum address.
    pub static ref ABDEL_ETHEREUM_ADDRESS: Address = Address::from_str("0x54b288676b749def5fc10eb17244fe2c87375de1").unwrap();
    /// Test value for proxy account class hash.
    pub static ref PROXY_ACCOUNT_CLASS_HASH: FieldElement =
        FieldElement::from_hex_be("0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5",).unwrap();
}
