#![allow(non_snake_case, clippy::derive_partial_eq_without_eq)]
pub mod kakarot_core;
pub mod relayer;

use cainome::rs::abigen_legacy;
use starknet::core::types::Felt;
use std::sync::LazyLock;

abigen_legacy!(ERC20, "./.kakarot/artifacts/ERC20.json");

/// Starknet native token address
pub static STARKNET_NATIVE_TOKEN: LazyLock<Felt> =
    LazyLock::new(|| Felt::from_hex("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap());
