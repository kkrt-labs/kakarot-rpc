#![allow(non_snake_case)]
pub mod kakarot;

use starknet_abigen_macros::abigen_legacy;
use starknet_abigen_parser;
use starknet_crypto::FieldElement;

abigen_legacy!(ERC20, "./artifacts/fixtures/ERC20.json");
