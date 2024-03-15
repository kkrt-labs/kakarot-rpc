use reth_primitives::{Address, B256, U256, U64};
use starknet::core::types::FieldElement;

#[derive(Debug, thiserror::Error)]
#[error("conversion failed")]
pub struct ConversionError;

#[derive(Clone, Debug)]
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

#[allow(clippy::fallible_impl_from)]
impl From<Address> for Felt252Wrapper {
    fn from(address: Address) -> Self {
        let felt = FieldElement::from_byte_slice_be(address.as_slice()).unwrap(); // safe unwrap since H160 is 20 bytes
        Self(felt)
    }
}

impl From<U64> for Felt252Wrapper {
    fn from(value: U64) -> Self {
        let felt = FieldElement::from(value.to::<u64>());
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
            return Err(ConversionError);
        }

        Ok(Self::from_slice(&bytes[12..]))
    }
}

impl TryFrom<B256> for Felt252Wrapper {
    type Error = ConversionError;

    fn try_from(value: B256) -> Result<Self, Self::Error> {
        let felt = FieldElement::from_bytes_be(value.as_ref()).map_err(|_| ConversionError)?;
        Ok(Self(felt))
    }
}

impl TryFrom<U256> for Felt252Wrapper {
    type Error = ConversionError;

    fn try_from(u256: U256) -> Result<Self, Self::Error> {
        let felt = FieldElement::from_bytes_be(&u256.to_be_bytes()).map_err(|_| ConversionError)?;
        Ok(Self(felt))
    }
}

impl From<Felt252Wrapper> for U256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        Self::from_be_bytes(felt.to_bytes_be())
    }
}

/// This macro provides a convenient way to convert a value from a source
/// type $val that implements Into<Felt252Wrapper> into a target type that
/// implements From<Felt252Wrapper>.
#[macro_export]
macro_rules! into_via_wrapper {
    ($val: expr) => {{
        let intermediate: Felt252Wrapper = $val.into();
        intermediate.into()
    }};
}

/// This macro provides a convenient way to convert a value from a source
/// type $val that implements TryInto<Felt252Wrapper> into a target type that
/// implements From<Felt252Wrapper>.
#[macro_export]
macro_rules! into_via_try_wrapper {
    ($val: expr) => {{
        let intermediate: Result<_, $crate::models::felt::ConversionError> =
            TryInto::<$crate::models::felt::Felt252Wrapper>::try_into($val)
                .map_err(|_| $crate::models::felt::ConversionError)
                .map(Into::into);
        intermediate
    }};
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
    fn test_felt_try_from_b256_should_pass() {
        // Given
        let hash = B256::from_slice(&FieldElement::MAX.to_bytes_be());

        // When
        let hash = Felt252Wrapper::try_from(hash).unwrap();

        // Then
        let expected_hash = FieldElement::MAX;
        assert_eq!(expected_hash, hash.0);
    }

    #[test]
    #[should_panic(expected = "Felt252WrapperConversionError")]
    fn test_felt_try_from_b256_should_fail() {
        // Given
        let hash = B256::from_str(OVERFLOW_FELT).unwrap();

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
    #[should_panic(expected = "Felt252WrapperConversionError")]
    fn test_felt_try_from_u256_should_fail() {
        // Given
        let hash = U256::from_str_radix(OVERFLOW_FELT, 16).unwrap();

        // When
        Felt252Wrapper::try_from(hash).unwrap();
    }
}
