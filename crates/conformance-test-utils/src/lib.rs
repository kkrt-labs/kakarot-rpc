pub mod kakarot;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use starknet::core::serde::unsigned_field_element::UfeHex;
use starknet::core::types::FieldElement;

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Felt(#[serde_as(as = "UfeHex")] pub FieldElement);
