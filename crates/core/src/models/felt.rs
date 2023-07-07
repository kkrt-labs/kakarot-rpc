use reth_primitives::{Address, H256, U256};
use starknet::core::types::FieldElement;

use super::ConversionError;

#[derive(Clone)]
pub struct Felt252Wrapper(FieldElement);

impl Felt252Wrapper {
    /// Troncate the first 12 bytes of the `FieldElement` and return the last 20 bytes as an
    /// Ethereum address. This is used to convert Starknet addresses to Ethereum addresses in
    /// cases where the Starknet address does not represent a Kakarot address, i.e. it does not have
    /// a `get_evm_address()` entrypoint.
    pub fn troncate_to_ethereum_address(&self) -> Address {
        let bytes = self.0.to_bytes_be();
        Address::from_slice(&bytes[12..])
    }
}

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
    type Error = ConversionError;

    fn try_from(felt: Felt252Wrapper) -> Result<Self, Self::Error> {
        let felt: FieldElement = felt.into();
        let bytes = felt.to_bytes_be();

        // Check if the first 12 bytes are all zeros.
        if bytes[0..12].iter().any(|&x| x != 0) {
            return Err(ConversionError::ToEthereumAddressError);
        }

        Ok(Address::from_slice(&bytes[12..]))
    }
}

impl TryFrom<H256> for Felt252Wrapper {
    type Error = ConversionError;

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
    type Error = ConversionError;

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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use hex::FromHex;

    use super::*;

    // 2**160 - 1
    const MAX_ADDRESS: &str = "ffffffffffffffffffffffffffffffffffffffff";
    // 2**160
    const OVERFLOW_ADDRESS: &str = "010000000000000000000000000000000000000000";

    // 2**251 + 17 * 2**192 + 1
    const OVERFLOW_FELT: &str = "0800000000000011000000000000000000000000000000000000000000000001";

    #[test]
    fn test_address_try_from_felt_should_pass() {
        // Given
        let address: Felt252Wrapper = FieldElement::from_hex_be(MAX_ADDRESS).unwrap().into();

        // When
        let address = Address::try_from(address).unwrap();

        // Then
        let expected_address = <[u8; 20]>::from_hex(MAX_ADDRESS).unwrap();
        assert_eq!(expected_address, address.0);
    }

    #[test]
    #[should_panic(expected = "ToEthereumAddressError")]
    fn test_address_try_from_felt_should_fail() {
        // Given
        let address: Felt252Wrapper = FieldElement::from_hex_be(OVERFLOW_ADDRESS).unwrap().into();

        // When
        Address::try_from(address).unwrap();
    }

    #[test]
    fn test_felt_try_from_h256_should_pass() {
        // Given
        let hash = H256::from_slice(&FieldElement::MAX.to_bytes_be());

        // When
        let hash = Felt252Wrapper::try_from(hash).unwrap();

        // Then
        let expected_hash = FieldElement::MAX;
        assert_eq!(expected_hash, hash.0);
    }

    #[test]
    #[should_panic(expected = "Felt252WrapperConversionError")]
    fn test_felt_try_from_h256_should_fail() {
        // Given
        let hash = H256::from_str(OVERFLOW_FELT).unwrap();

        // When
        Felt252Wrapper::try_from(hash).unwrap();
    }

    #[test]
    fn test_felt_try_from_u256_should_pass() {
        // Given
        let hash = U256::try_from_be_slice(&FieldElement::MAX.to_bytes_be()).unwrap();

        // When
        let hash = Felt252Wrapper::try_from(hash).unwrap();

        // Then
        let expected_hash = FieldElement::MAX;
        assert_eq!(expected_hash, hash.0);
    }

    #[test]
    fn test_felt_try_from_u256_should_fail() {
        // Given
        let hash = U256::from_str_radix(OVERFLOW_FELT, 16).unwrap();

        // When
        let hash = Felt252Wrapper::try_from(hash).err().unwrap();

        // Then
        match hash {
            ConversionError::Felt252WrapperConversionError(err) => {
                assert_eq!("number out of range", err.to_string());
            }
            _ => panic!("Expected ConversionError::ToEthereumAddressError"),
        }
    }
}
