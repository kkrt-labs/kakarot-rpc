use crate::providers::eth_provider::utils::split_u256;
use alloy_primitives::Bytes;
use blockifier::abi::{abi_utils::get_storage_var_address, sierra_types::next_storage_key};
use reth_primitives::{alloy_primitives::keccak256, Address, KECCAK_EMPTY, U256};
use revm_interpreter::analysis::to_analysed;
use revm_primitives::Bytecode;
use starknet::core::utils::cairo_short_string_to_felt;
use starknet_api::{core::Nonce, state::StorageKey, StarknetApiError};
use starknet_crypto::{poseidon_permute_comp, Felt};

pub const ACCOUNT_BYTECODE_LEN: &str = "Account_bytecode_len";
pub const ACCOUNT_CODE_HASH: &str = "Account_code_hash";
pub const ACCOUNT_EVM_ADDRESS: &str = "Account_evm_address";
pub const ACCOUNT_IS_INITIALIZED: &str = "Account_is_initialized";
pub const ACCOUNT_NONCE: &str = "Account_nonce";
pub const ACCOUNT_STORAGE: &str = "Account_storage";
pub const ACCOUNT_VALID_JUMPDESTS: &str = "Account_valid_jumpdests";

/// An account.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Account {
    /// Balance.
    pub balance: U256,
    /// Code.
    pub code: Bytes,
    /// Nonce.
    pub nonce: U256,
    /// Storage.
    pub storage: Vec<(U256, U256)>,
}

#[macro_export]
macro_rules! starknet_storage {
    ($storage_var: expr, $felt: expr) => {
        (
            get_storage_var_address($storage_var, &[]),
            Felt::from($felt),
        )
    };
    ($storage_var: expr, [$($key: expr),*], $felt: expr) => {
        {
            let args = vec![$($key),*];
            (
                get_storage_var_address($storage_var, &args),
                Felt::from($felt),
            )
        }
    };
}

/// Structure representing a Kakarot account.
/// Contains a nonce, Starknet storage, account
/// type, evm address and starknet address.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct KakarotAccount {
    pub(crate) evm_address: Felt,
    pub(crate) nonce: Nonce,
    pub(crate) storage: Vec<(StorageKey, Felt)>,
}

impl KakarotAccount {
    pub const fn evm_address(&self) -> &Felt {
        &self.evm_address
    }

    pub const fn nonce(&self) -> &Nonce {
        &self.nonce
    }

    pub fn storage(&self) -> &[(StorageKey, Felt)] {
        self.storage.as_slice()
    }
}

#[derive(Debug, Default, Clone)]
pub enum AccountType {
    #[default]
    Uninitialized = 0,
    EOA = 1,
    Contract = 2,
}

impl KakarotAccount {
    pub fn new(evm_address: &Address, account: Account) -> Result<Self, StarknetApiError> {
        let nonce = Felt::from(
            TryInto::<u128>::try_into(account.nonce)
                .map_err(|err| StarknetApiError::OutOfRange { string: err.to_string() })?,
        );

        let evm_address = Felt::from_bytes_be_slice(&evm_address.0[..]);

        let mut storage = vec![
            starknet_storage!(ACCOUNT_EVM_ADDRESS, evm_address),
            starknet_storage!(ACCOUNT_IS_INITIALIZED, 1u8),
            starknet_storage!(ACCOUNT_BYTECODE_LEN, account.code.len() as u32),
            starknet_storage!(ACCOUNT_NONCE, nonce),
        ];

        // Initialize the bytecode storage var.
        let mut bytecode_storage = pack_byte_array_to_starkfelt_array(&account.code)
            .enumerate()
            .map(|(i, bytes)| (StorageKey::from(i as u32), bytes))
            .collect();
        storage.append(&mut bytecode_storage);

        // Initialize the code hash var
        let account_is_empty = account.code.is_empty() && nonce == Felt::ZERO && account.balance == U256::ZERO;
        let code_hash = if account_is_empty {
            U256::ZERO
        } else if account.code.is_empty() {
            U256::from_be_slice(KECCAK_EMPTY.as_slice())
        } else {
            U256::from_be_slice(keccak256(account.code.clone()).as_slice())
        };

        let code_hash_values = split_u256(code_hash);
        let code_hash_low_key = get_storage_var_address(ACCOUNT_CODE_HASH, &[]);
        let code_hash_high_key = next_storage_key(&code_hash_low_key)?;
        storage.extend([
            (code_hash_low_key, Felt::from(code_hash_values[0])),
            (code_hash_high_key, Felt::from(code_hash_values[1])),
        ]);

        // Initialize the bytecode jumpdests.
        let bytecode = to_analysed(Bytecode::new_raw(account.code));
        let valid_jumpdests: Vec<usize> = match bytecode {
            Bytecode::LegacyAnalyzed(legacy_analyzed_bytecode) => legacy_analyzed_bytecode
                .jump_table()
                .0
                .iter()
                .enumerate()
                .filter_map(|(index, bit)| bit.as_ref().then(|| index))
                .collect(),
            _ => unreachable!("Bytecode should be analysed"),
        };

        let jumdpests_storage_address = get_storage_var_address(ACCOUNT_VALID_JUMPDESTS, &[]);
        let jumdpests_storage_address = Felt::from(jumdpests_storage_address);
        valid_jumpdests.into_iter().for_each(|index| {
            storage.push(((jumdpests_storage_address + Felt::from(index)).try_into().unwrap(), Felt::ONE))
        });

        // Initialize the storage vars.
        let mut evm_storage_storage: Vec<(StorageKey, Felt)> = account
            .storage
            .iter()
            .flat_map(|(k, v)| {
                let keys = split_u256(*k).map(Into::into);
                let values = split_u256(*v).map(Into::<Felt>::into);
                let low_key = get_storage_var_address(ACCOUNT_STORAGE, &keys);
                let high_key = next_storage_key(&low_key).unwrap(); // can fail only if low is the max key
                vec![(low_key, values[0]), (high_key, values[1])]
            })
            .collect();
        storage.append(&mut evm_storage_storage);

        Ok(Self { storage, evm_address, nonce: Nonce(nonce) })
    }
}

/// Splits a byte array into 31-byte chunks and converts each chunk to a Felt.
pub fn pack_byte_array_to_starkfelt_array(bytes: &[u8]) -> impl Iterator<Item = Felt> + '_ {
    bytes.chunks(31).map(Felt::from_bytes_be_slice)
}

/// Computes the inner pointer of a byte array in storage.
///
/// The pointer is determined by the hash of:
/// - The base address of the byte array.
/// - The storage segment.
/// - The short string `ByteArray`.
///
/// # Arguments
/// * `base_address` - The base address of the byte array.
/// * `storage_segment` - The index of the storage segment to compute the pointer for. Each segment should store at most 256 * 31 bytes
///
/// # Returns
/// The inner pointer of the byte array.
pub fn inner_byte_array_pointer(base_address: Felt, storage_segment: Felt) -> Felt {
    let suffix = cairo_short_string_to_felt("ByteArray").unwrap();
    let mut state = [base_address, storage_segment, suffix];
    poseidon_permute_comp(&mut state);
    state[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use blockifier::abi::abi_utils::get_storage_var_address;
    use reth_primitives::Bytes;

    const ACCOUNT_BYTECODE: &str = "Account_bytecode";

    #[test]
    fn test_pack_byte_array_to_starkfelt_array() {
        // Given
        let bytes = Bytes::from([0x01, 0x02, 0x03, 0x04, 0x05]);

        // When
        let result: Vec<_> = pack_byte_array_to_starkfelt_array(&bytes).collect();

        // Then
        assert_eq!(result, vec![Felt::from(0x0001_0203_0405_u64)]);
    }

    #[test]
    fn test_inner_byte_array_pointer() {
        // Given
        let base_address: Felt = get_storage_var_address(ACCOUNT_BYTECODE, &[]).into();
        let chunk = Felt::ZERO;

        // When
        let result = inner_byte_array_pointer(base_address, chunk);

        // Then
        assert_eq!(
            result,
            Felt::from_hex("0x030dc4fd6786155d4743a0f56ea73bea9521eba2552a2ca5080b830ad047907a").unwrap()
        );
    }
}
