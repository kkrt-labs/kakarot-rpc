#![allow(non_snake_case, clippy::derive_partial_eq_without_eq)]
pub mod kakarot_core;

use cainome::rs::abigen_legacy;
use lazy_static::lazy_static;
use starknet::core::types::Felt;

abigen_legacy!(ERC20, "./.kakarot/artifacts/fixtures/ERC20.json");

lazy_static! {
    pub static ref STARKNET_NATIVE_TOKEN: Felt =
        Felt::from_hex("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap();
}
