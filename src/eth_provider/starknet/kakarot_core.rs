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
    use reth_primitives::{hex, transaction::TxEip2930, Bytes, Signature, TxKind, U256};

    #[test]
    fn test_to_starknet_transaction() {
        // Define a sample signed transaction.
        // Using https://sepolia.kakarotscan.org/tx/0x5be347c9eb86cf04b884c7e6f432c6daa2054b46c3c70c7d4536e4c009765abe
        let transaction = TransactionSigned::from_transaction_and_signature(Transaction::Eip2930(TxEip2930 { chain_id: 1_802_203_764, nonce: 33, gas_price:  0, gas_limit: 302_606, to: TxKind::Create, value: U256::ZERO, access_list: Default::default(), input: Bytes::from_str("0x608060405260405161040a38038061040a83398101604081905261002291610268565b61002c8282610033565b5050610352565b61003c82610092565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b90600090a280511561008657610081828261010e565b505050565b61008e610185565b5050565b806001600160a01b03163b6000036100cd57604051634c9c8ce360e01b81526001600160a01b03821660048201526024015b60405180910390fd5b7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc80546001600160a01b0319166001600160a01b0392909216919091179055565b6060600080846001600160a01b03168460405161012b9190610336565b600060405180830381855af49150503d8060008114610166576040519150601f19603f3d011682016040523d82523d6000602084013e61016b565b606091505b50909250905061017c8583836101a6565b95945050505050565b34156101a45760405163b398979f60e01b815260040160405180910390fd5b565b6060826101bb576101b682610205565b6101fe565b81511580156101d257506001600160a01b0384163b155b156101fb57604051639996b31560e01b81526001600160a01b03851660048201526024016100c4565b50805b9392505050565b8051156102155780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b634e487b7160e01b600052604160045260246000fd5b60005b8381101561025f578181015183820152602001610247565b50506000910152565b6000806040838503121561027b57600080fd5b82516001600160a01b038116811461029257600080fd5b60208401519092506001600160401b03808211156102af57600080fd5b818501915085601f8301126102c357600080fd5b8151818111156102d5576102d561022e565b604051601f8201601f19908116603f011681019083821181831017156102fd576102fd61022e565b8160405282815288602084870101111561031657600080fd5b610327836020830160208801610244565b80955050505050509250929050565b60008251610348818460208701610244565b9190910192915050565b60aa806103606000396000f3fe6080604052600a600c565b005b60186014601a565b6051565b565b6000604c7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b905090565b3660008037600080366000845af43d6000803e808015606f573d6000f35b3d6000fdfea2646970667358221220d0232cfa81216c3e4973e570f043b57ccb69ae4a81b8bc064338713721c87a9f64736f6c6343000814003300000000000000000000000009635f643e140090a9a8dcd712ed6285858cebef000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000647a1ac61e00000000000000000000000084ea74d481ee0a5332c457a4d796187f6ba67feb00000000000000000000000000000000000000000000000000038d7ea4c68000000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000").unwrap() }), Signature { r: U256::from_str("0x6290c177b6ee7b16d87909474a792d9ac022385505161e91191c57d666b61496").unwrap(), s: U256::from_str("0x7ba95168843acb8b888de596c28033c6c66a9cb6c7621cfc996bc5851115634d").unwrap(), odd_y_parity: true });

        // Invoke the function to convert the transaction to Starknet format.
        match to_starknet_transaction(
            &transaction,
            1_802_203_764,
            Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap(),
            0,
        )
        .unwrap()
        {
            // Handle the Starknet transaction format.
            BroadcastedInvokeTransaction::V1(tx) => {
                // Transaction signature assertion.
                assert_eq!(
                    tx.signature,
                    vec![
                        FieldElement::from(255_389_455_834_799_815_707_633_470_637_690_197_142_u128),
                        FieldElement::from(131_015_958_324_370_192_097_986_834_655_742_602_650_u128),
                        FieldElement::from(263_740_705_169_547_910_390_939_684_488_449_712_973_u128),
                        FieldElement::from(164_374_192_806_466_935_713_473_791_294_001_132_486_u128),
                        FieldElement::ONE
                    ]
                );

                // Transaction nonce assertion.
                assert_eq!(tx.nonce, FieldElement::from(33_u128));

                // Assertion for transaction properties.
                assert!(!tx.is_query);
                assert_eq!(tx.max_fee, FieldElement::ZERO);

                // Assert the length of calldata.
                assert_eq!(tx.calldata.len(), transaction.transaction.length() + 6);

                // Assert the first 6 elements of calldata.
                assert_eq!(
                    tx.calldata[0..6],
                    vec![
                        FieldElement::ONE,
                        *KAKAROT_ADDRESS,
                        *ETH_SEND_TRANSACTION,
                        FieldElement::ZERO,
                        FieldElement::from(transaction.transaction.length()),
                        FieldElement::from(transaction.transaction.length()),
                    ]
                );

                // Assert the sender address.
                assert_eq!(
                    tx.sender_address,
                    get_contract_address(
                        FieldElement::from(Felt252Wrapper::from(
                            Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()
                        )),
                        *UNINITIALIZED_ACCOUNT_CLASS_HASH,
                        &[
                            *KAKAROT_ADDRESS,
                            FieldElement::from(Felt252Wrapper::from(
                                Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()
                            )),
                        ],
                        FieldElement::ZERO,
                    )
                )
            }
            _ => panic!("Invalid Starknet broadcasted transaction"),
        }
    }

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
        to_starknet_transaction(&transaction, 1, transaction.recover_signer().unwrap(), 100_000_000).unwrap();
    }
}
