use std::str::FromStr;

#[cfg(not(feature = "hive"))]
use crate::eth_provider::error::EthApiError;
use crate::models::felt::Felt252Wrapper;
use alloy_rlp::Encodable;
use cainome::rs::abigen_legacy;
use dotenvy::dotenv;
use lazy_static::lazy_static;
use reth_primitives::{Address, Transaction, TransactionSigned};
use starknet::{
    core::{
        types::{BroadcastedInvokeTransaction, BroadcastedInvokeTransactionV1},
        utils::get_contract_address,
    },
    macros::selector,
};
use starknet_crypto::FieldElement;

use crate::{
    eth_provider::{provider::EthProviderResult, utils::split_u256},
    into_via_wrapper,
};

// Contract ABIs

pub mod account_contract {
    use super::*;
    abigen_legacy!(AccountContract, "./.kakarot/artifacts/account_contract.json");
}

#[allow(clippy::too_many_arguments)]
pub mod core {
    use super::*;
    abigen_legacy!(KakarotCore, "./.kakarot/artifacts/kakarot.json");

    #[derive(Debug)]
    pub struct CallInput {
        pub(crate) nonce: FieldElement,
        pub(crate) from: FieldElement,
        pub(crate) to: self::Option,
        pub(crate) gas_limit: FieldElement,
        pub(crate) gas_price: FieldElement,
        pub(crate) value: Uint256,
        pub(crate) calldata: Vec<FieldElement>,
    }
}

fn env_var_to_field_element(var_name: &str) -> FieldElement {
    dotenv().ok();
    let env_var = std::env::var(var_name).unwrap_or_else(|_| panic!("Missing environment variable {var_name}"));

    FieldElement::from_str(&env_var).unwrap_or_else(|_| panic!("Invalid hex string for {var_name}"))
}

lazy_static! {
    // Contract addresses
    pub static ref KAKAROT_ADDRESS: FieldElement = env_var_to_field_element("KAKAROT_ADDRESS");

    // Contract class hashes
    pub static ref UNINITIALIZED_ACCOUNT_CLASS_HASH: FieldElement = env_var_to_field_element("UNINITIALIZED_ACCOUNT_CLASS_HASH");
    pub static ref CONTRACT_ACCOUNT_CLASS_HASH: FieldElement = env_var_to_field_element("CONTRACT_ACCOUNT_CLASS_HASH");

    // Contract selectors
    pub static ref ETH_SEND_TRANSACTION: FieldElement = selector!("eth_send_transaction");

    // Maximum number of felts (bytes) in calldata
    pub static ref MAX_FELTS_IN_CALLDATA: usize = usize::from_str(
        &std::env::var("MAX_FELTS_IN_CALLDATA")
            .unwrap_or_else(|_| panic!("Missing environment variable MAX_FELTS_IN_CALLDATA"))
    ).expect("failing to parse MAX_FELTS_IN_CALLDATA");
}

// Kakarot utils
/// Compute the starknet address given a eth address
#[inline]
pub fn starknet_address(address: Address) -> FieldElement {
    let evm_address = into_via_wrapper!(address);
    get_contract_address(
        evm_address,
        *UNINITIALIZED_ACCOUNT_CLASS_HASH,
        &[*KAKAROT_ADDRESS, evm_address],
        FieldElement::ZERO,
    )
}

/// Convert a Ethereum transaction into a Starknet transaction
pub fn to_starknet_transaction(
    transaction: &TransactionSigned,
    chain_id: u64,
    signer: Address,
    max_fee: u64,
) -> EthProviderResult<BroadcastedInvokeTransaction> {
    let sender_address = starknet_address(signer);

    // Step: Signature
    // Extract the signature from the Ethereum Transaction
    // and place it in the Starknet signature InvokeTransaction vector
    let signature: Vec<FieldElement> = {
        let transaction_signature = transaction.signature();

        let mut signature = Vec::with_capacity(5);
        signature.extend_from_slice(&split_u256(transaction_signature.r));
        signature.extend_from_slice(&split_u256(transaction_signature.s));

        // Push the last element of the signature
        // In case of a Legacy Transaction, it is v := {0, 1} + chain_id * 2 + 35
        // Else, it is odd_y_parity
        if let Transaction::Legacy(_) = transaction.transaction {
            signature.push(transaction_signature.v(Some(chain_id)).into());
        } else {
            signature.push((transaction_signature.odd_y_parity as u64).into());
        }

        signature
    };

    // Step: Calldata
    // RLP encode the transaction without the signature
    // Example: For Legacy Transactions: rlp([nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0])
    let mut signed_data = Vec::with_capacity(transaction.transaction.length());
    transaction.transaction.encode_without_signature(&mut signed_data);

    // Prepare the calldata for the Starknet invoke transaction
    let capacity = 6 + signed_data.len();

    // Check if call data is too large
    #[cfg(not(feature = "hive"))]
    if capacity > *MAX_FELTS_IN_CALLDATA {
        return Err(EthApiError::CalldataExceededLimit(*MAX_FELTS_IN_CALLDATA as u64, capacity as u64));
    }

    let mut calldata = Vec::with_capacity(capacity);
    calldata.append(&mut vec![
        FieldElement::ONE,        // call array length
        *KAKAROT_ADDRESS,         // contract address
        *ETH_SEND_TRANSACTION,    // selector
        FieldElement::ZERO,       // data offset
        signed_data.len().into(), // data length
        signed_data.len().into(), // calldata length
    ]);
    calldata.append(&mut signed_data.into_iter().map(Into::into).collect());

    Ok(BroadcastedInvokeTransaction::V1(BroadcastedInvokeTransactionV1 {
        max_fee: max_fee.into(),
        signature,
        nonce: transaction.nonce().into(),
        sender_address,
        calldata,
        is_query: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rlp::Decodable;
    use reth_primitives::hex;

    #[test]
    #[should_panic(expected = "calldata exceeded limit of 22500: 30032")]
    fn to_starknet_transaction_too_large_calldata_test() {
        // Test that an example create transaction from goerli decodes properly
        let tx_bytes = hex!("b901f202f901ee05228459682f008459682f11830209bf8080b90195608060405234801561001057600080fd5b50610175806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c80630c49c36c14610030575b600080fd5b61003861004e565b604051610045919061011d565b60405180910390f35b60606020600052600f6020527f68656c6c6f2073746174656d696e64000000000000000000000000000000000060405260406000f35b600081519050919050565b600082825260208201905092915050565b60005b838110156100be5780820151818401526020810190506100a3565b838111156100cd576000848401525b50505050565b6000601f19601f8301169050919050565b60006100ef82610084565b6100f9818561008f565b93506101098185602086016100a0565b610112816100d3565b840191505092915050565b6000602082019050818103600083015261013781846100e4565b90509291505056fea264697066735822122051449585839a4ea5ac23cae4552ef8a96b64ff59d0668f76bfac3796b2bdbb3664736f6c63430008090033c080a0136ebffaa8fc8b9fda9124de9ccb0b1f64e90fbd44251b4c4ac2501e60b104f9a07eb2999eec6d185ef57e91ed099afb0a926c5b536f0155dd67e537c7476e1471");

        // Decode the transaction from the provided bytes
        let mut transaction = TransactionSigned::decode(&mut &tx_bytes[..]).unwrap();

        // Set the input of the transaction to be a vector of 30,000 zero bytes
        transaction.transaction.set_input(vec![0; 30000].into());

        // Attempt to convert the transaction into a Starknet transaction
        to_starknet_transaction(&transaction, 1, transaction.recover_signer().unwrap(), 100000000).unwrap();
    }
}
