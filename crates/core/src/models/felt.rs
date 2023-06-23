use reth_primitives::{H256, U256};
use starknet::core::types::FieldElement;

pub struct Felt252Wrapper(FieldElement);

impl From<FieldElement> for Felt252Wrapper {
    fn from(felt: FieldElement) -> Self {
        Self(felt)
    }
}

impl From<Felt252Wrapper> for FieldElement {
    fn from(felt: Felt252Wrapper) -> Self {
        felt.0
    }
}

impl From<Felt252Wrapper> for H256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        H256::from_slice(&felt.to_bytes_be())
    }
}

impl From<Felt252Wrapper> for U256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        U256::from_be_bytes(felt.to_bytes_be())
    }
}
