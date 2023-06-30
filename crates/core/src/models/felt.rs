use reth_primitives::{Address, H256, U256};
use starknet::core::types::{FieldElement, FromByteArrayError};
use thiserror::Error;

use crate::client::errors::EthApiError;

#[derive(Debug, Error)]
pub enum Felt252WrapperError {
    #[error(transparent)]
    FromByteArrayError(#[from] FromByteArrayError),
    #[error(
        "Failed to convert Felt252Wrapper to Ethereum address: the value exceeds the maximum size of an Ethereum \
         address"
    )]
    ToEthereumAddressError,
}

impl From<Felt252WrapperError> for EthApiError {
    fn from(err: Felt252WrapperError) -> Self {
        EthApiError::ConversionError(err.into())
    }
}

#[derive(Clone)]
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

impl From<u64> for Felt252Wrapper {
    fn from(u64: u64) -> Self {
        let felt = FieldElement::from(u64);
        Self(felt)
    }
}

impl From<Address> for Felt252Wrapper {
    fn from(address: Address) -> Self {
        let felt = FieldElement::from_byte_slice_be(&address.0).unwrap(); // safe unwrap since H160 is 20 bytes
        Self(felt)
    }
}

impl TryFrom<Felt252Wrapper> for Address {
    type Error = Felt252WrapperError;

    fn try_from(felt: Felt252Wrapper) -> Result<Self, Self::Error> {
        let felt: FieldElement = felt.into();
        let bytes = felt.to_bytes_be();

        // Check if the first 12 bytes are all zeros.
        if bytes[0..12].iter().any(|&x| x != 0) {
            return Err(Felt252WrapperError::ToEthereumAddressError);
        }

        Ok(Address::from_slice(&bytes[12..]))
    }
}

impl TryFrom<H256> for Felt252Wrapper {
    type Error = Felt252WrapperError;

    fn try_from(h256: H256) -> Result<Self, Self::Error> {
        let felt = FieldElement::from_bytes_be(&h256)?;
        Ok(Self(felt))
    }
}

impl From<Felt252Wrapper> for H256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        H256::from_slice(&felt.to_bytes_be())
    }
}

impl TryFrom<U256> for Felt252Wrapper {
    type Error = Felt252WrapperError;

    fn try_from(u256: U256) -> Result<Self, Self::Error> {
        let felt = FieldElement::from_bytes_be(&u256.to_be_bytes())?;
        Ok(Self(felt))
    }
}

impl From<Felt252Wrapper> for U256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        U256::from_be_bytes(felt.to_bytes_be())
    }
}
