#![allow(non_snake_case)]
pub mod kakarot_core;

use cainome::rs::abigen_legacy;
use lazy_static::lazy_static;
use starknet_crypto::FieldElement;

abigen_legacy!(ERC20, ".kakarot/artifacts/fixtures/ERC20.json");

lazy_static! {
    pub static ref STARKNET_NATIVE_TOKEN: FieldElement =
        FieldElement::from_hex_be("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap();
}
