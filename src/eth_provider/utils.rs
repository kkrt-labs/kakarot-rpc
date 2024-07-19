use cainome::cairo_serde::Error;
use reth_primitives::{U128, U256};
use starknet::{
    core::types::{ContractErrorData, StarknetError},
    providers::ProviderError,
};

/// Splits a U256 value into two generic values implementing the From<u128> trait
#[inline]
pub fn split_u256<T: From<u128>>(value: impl Into<U256>) -> [T; 2] {
    let value: U256 = value.into();
    let low: u128 = (value & U256::from(U128::MAX)).try_into().unwrap(); // safe to unwrap
    let high: U256 = value >> 128;
    let high: u128 = high.try_into().unwrap(); // safe to unwrap
    [T::from(low), T::from(high)]
}

/// Checks if the error is a contract not found error.
/// Some providers return a contract not found error when the contract is not deployed.
/// Katana returns a contract error with a revert message containing "is not deployed".
#[inline]
pub(crate) fn contract_not_found<T>(err: &Result<T, Error>) -> bool {
    match err {
        Ok(_) => false,
        Err(err) => {
            matches!(err, Error::Provider(ProviderError::StarknetError(StarknetError::ContractNotFound)))
                || matches!(
                    err,
                    Error::Provider(ProviderError::StarknetError(StarknetError::ContractError(ContractErrorData {
                        revert_error: reason
                    }))) if reason.contains("is not deployed")
                )
        }
    }
}

#[inline]
pub(crate) fn class_hash_not_declared<T>(err: &Result<T, Error>) -> bool {
    match err {
        Ok(_) => false,
        Err(err) => {
            matches!(
                err,
                Error::Provider(ProviderError::StarknetError(StarknetError::ContractError(ContractErrorData {
                    revert_error
                }))) if revert_error.contains("is not declared.")
            )
        }
    }
}

/// Checks if the error is an entrypoint not found error.
#[inline]
pub(crate) fn entrypoint_not_found<T>(err: &Result<T, Error>) -> bool {
    match err {
        Ok(_) => false,
        Err(err) => matches!(
            err,
            Error::Provider(ProviderError::StarknetError(StarknetError::ContractError(ContractErrorData {
                revert_error: reason
            }))) if reason.contains("Entry point") && reason.contains("not found in contract")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::str::FromStr;

    #[test]
    fn test_split_u256() {
        // Define a property-based test using Proptest
        proptest!(|(value in any::<U256>())| {
            // Call the split_u256 function to split the U256 value into two u128 values
            let result = split_u256::<u128>(value);

            // Combine the two u128 values into a hexadecimal string
            let combined_hex = format!("{:#x}{:0width$x}", result[1], result[0], width = 32);

            // Assertion to check the equality with the original U256 value
            assert_eq!(U256::from_str(&combined_hex).unwrap(), value);
        });
    }

    #[test]
    fn test_class_hash_not_declared() {
        let err = Error::Provider(ProviderError::StarknetError(StarknetError::ContractError(ContractErrorData {
            revert_error: "\"Error in the called contract (0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7):
                           \\nError at pc=0:104:\\nGot an exception while executing a hint: Class with ClassHash(\\n    StarkFelt(\\n
                            \\\"0x0000000000000000000000000000000000000000000000000000000000000000\\\",\\n    ),\\n) is not declared.\\nCairo traceback (most recent call last):\\n
                            Unknown location (pc=0:1678)\\nUnknown location (pc=0:1664)\\n\"".to_string(),
        })));

        assert!(class_hash_not_declared::<()>(&Err(err)));
    }
}
