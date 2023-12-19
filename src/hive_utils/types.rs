use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use starknet::core::serde::unsigned_field_element::UfeHex;
use starknet::core::types::FieldElement;

/// A wrapper around a `FieldElement` that serializes it as a hex string.
#[serde_as]
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Felt(#[serde_as(as = "UfeHex")] pub FieldElement);

/// [`Felt`] from [`FieldElement`].
impl From<FieldElement> for Felt {
    fn from(fe: FieldElement) -> Self {
        Self(fe)
    }
}

/// Type wrapper for a contract address.
pub type ContractAddress = Felt;

/// Type wrapper for a storage key;
pub type StorageKey = Felt;

/// Type wrapper for a storage value.
pub type StorageValue = Felt;

/// Type wrapper for a class hash.
pub type ClassHash = Felt;

/// Type wrapper for a contract storage key.
pub type ContractStorageKey = (ContractAddress, StorageKey);
