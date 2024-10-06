use crate::providers::eth_provider::{
    provider::EthApiResult,
    starknet::kakarot_core::{ETH_SEND_TRANSACTION, KAKAROT_ADDRESS},
    utils::split_u256,
};
use alloy_rlp::Encodable;
use reth_primitives::{transaction::legacy_parity, Transaction, TransactionSigned};
use starknet::core::types::Felt;
#[cfg(not(feature = "hive"))]
use {
    crate::providers::eth_provider::error::EthApiError,
    crate::providers::eth_provider::starknet::kakarot_core::MAX_FELTS_IN_CALLDATA,
};

/// Returns the transaction's signature as a [`Vec<Felt>`].
/// Fields r and s are split into two 16-bytes chunks both converted
/// to [`Felt`].
pub(crate) fn transaction_signature_to_field_elements(transaction_signed: &TransactionSigned) -> Vec<Felt> {
    let transaction_signature = transaction_signed.signature();

    let mut signature = Vec::with_capacity(5);
    signature.extend_from_slice(&split_u256(transaction_signature.r()));
    signature.extend_from_slice(&split_u256(transaction_signature.s()));

    // Push the last element of the signature
    // In case of a Legacy Transaction, it is v := {0, 1} + chain_id * 2 + 35
    // or {0, 1} + 27 for pre EIP-155 transactions.
    // Else, it is odd_y_parity
    if let Transaction::Legacy(_) = transaction_signed.transaction {
        let chain_id = transaction_signed.chain_id();
        signature.push(legacy_parity(transaction_signature, chain_id).to_u64().into());
    } else {
        signature.push(transaction_signature.v().to_u64().into());
    }

    signature
}

/// Returns the transaction's data and signature combined into a
/// [`execute_from_outside`] type transaction. The payload still needs
/// to be signed by the relayer before broadcasting.
pub fn transaction_data_to_starknet_calldata(
    transaction_signed: &TransactionSigned,
    relayer_address: Felt,
) -> EthApiResult<Vec<Felt>> {
    let mut signed_data = Vec::with_capacity(transaction_signed.transaction.length());
    transaction_signed.transaction.encode_without_signature(&mut signed_data);

    // Extract the signature from the signed transaction
    let mut signature = transaction_signature_to_field_elements(transaction_signed);

    // Pack the calldata in 31-byte chunks
    let mut signed_data: Vec<Felt> = std::iter::once(Felt::from(signed_data.len()))
        .chain(signed_data.chunks(31).map(Felt::from_bytes_be_slice))
        .collect();

    // Prepare the calldata for the Starknet invoke transaction
    let capacity = 10 + signed_data.len() + signature.len() + 1;

    // Check if call data is too large
    #[cfg(not(feature = "hive"))]
    if capacity > *MAX_FELTS_IN_CALLDATA {
        return Err(EthApiError::CalldataExceededLimit(*MAX_FELTS_IN_CALLDATA, capacity));
    }

    let mut execute_from_outside_calldata = Vec::with_capacity(capacity);

    // Construct the execute from outside calldata
    // https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/accounts/account_contract.cairo#L73
    execute_from_outside_calldata.append(&mut vec![
        relayer_address,          // OutsideExecution caller
        Felt::ZERO,               // OutsideExecution nonce
        Felt::ZERO,               // OutsideExecution execute_after
        Felt::from(u32::MAX),     // OutsideExecution execute_before
        Felt::ONE,                // call_array_len
        *KAKAROT_ADDRESS,         // CallArray to
        *ETH_SEND_TRANSACTION,    // CallArray selector
        Felt::ZERO,               // CallArray data_offset
        signed_data.len().into(), // CallArray data_len
        signed_data.len().into(), // CallArray calldata_len
    ]);
    execute_from_outside_calldata.append(&mut signed_data);
    execute_from_outside_calldata.push(signature.len().into());
    execute_from_outside_calldata.append(&mut signature);

    Ok(execute_from_outside_calldata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::TxEip2930;
    use alloy_primitives::{bytes, hex, TxKind, U256};
    use alloy_rlp::Decodable;
    use reth_primitives::Signature;
    use std::str::FromStr;

    #[test]
    #[ignore = "failing because of relayer change"]
    fn test_transaction_data_to_starknet_calldata() {
        // Define a sample signed transaction.
        // Using https://sepolia.kakarotscan.org/tx/0x5be347c9eb86cf04b884c7e6f432c6daa2054b46c3c70c7d4536e4c009765abe
        let transaction = TransactionSigned::from_transaction_and_signature(Transaction::Eip2930(TxEip2930 {
            chain_id: 1_802_203_764,
            nonce: 33,
            gas_price: 0,
            gas_limit: 302_606,
            to: TxKind::Create,
            value: U256::ZERO,
            access_list: Default::default(),
            input: bytes!("608060405260405161040a38038061040a83398101604081905261002291610268565b61002c8282610033565b5050610352565b61003c82610092565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b90600090a280511561008657610081828261010e565b505050565b61008e610185565b5050565b806001600160a01b03163b6000036100cd57604051634c9c8ce360e01b81526001600160a01b03821660048201526024015b60405180910390fd5b7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc80546001600160a01b0319166001600160a01b0392909216919091179055565b6060600080846001600160a01b03168460405161012b9190610336565b600060405180830381855af49150503d8060008114610166576040519150601f19603f3d011682016040523d82523d6000602084013e61016b565b606091505b50909250905061017c8583836101a6565b95945050505050565b34156101a45760405163b398979f60e01b815260040160405180910390fd5b565b6060826101bb576101b682610205565b6101fe565b81511580156101d257506001600160a01b0384163b155b156101fb57604051639996b31560e01b81526001600160a01b03851660048201526024016100c4565b50805b9392505050565b8051156102155780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b634e487b7160e01b600052604160045260246000fd5b60005b8381101561025f578181015183820152602001610247565b50506000910152565b6000806040838503121561027b57600080fd5b82516001600160a01b038116811461029257600080fd5b60208401519092506001600160401b03808211156102af57600080fd5b818501915085601f8301126102c357600080fd5b8151818111156102d5576102d561022e565b604051601f8201601f19908116603f011681019083821181831017156102fd576102fd61022e565b8160405282815288602084870101111561031657600080fd5b610327836020830160208801610244565b80955050505050509250929050565b60008251610348818460208701610244565b9190910192915050565b60aa806103606000396000f3fe6080604052600a600c565b005b60186014601a565b6051565b565b6000604c7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b905090565b3660008037600080366000845af43d6000803e808015606f573d6000f35b3d6000fdfea2646970667358221220d0232cfa81216c3e4973e570f043b57ccb69ae4a81b8bc064338713721c87a9f64736f6c6343000814003300000000000000000000000009635f643e140090a9a8dcd712ed6285858cebef000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000647a1ac61e00000000000000000000000084ea74d481ee0a5332c457a4d796187f6ba67feb00000000000000000000000000000000000000000000000000038d7ea4c68000000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000")
        }),    Signature::from_rs_and_parity(U256::from_str("0x6290c177b6ee7b16d87909474a792d9ac022385505161e91191c57d666b61496").unwrap(), U256::from_str("0x7ba95168843acb8b888de596c28033c6c66a9cb6c7621cfc996bc5851115634d").unwrap(), true).expect("Failed to generate signature")

    );

        // Invoke the function to convert the transaction to Starknet format.
        let calldata = transaction_data_to_starknet_calldata(&transaction, Felt::ZERO).unwrap();

        // Assert the length of calldata.
        // We must adapt the check as we pack the calldata in 31-byte chunks.
        assert_eq!(calldata.len(), (transaction.transaction.length() + 30) / 31 + 1 + 6);

        // Assert the first 6 elements of calldata.
        assert_eq!(
            calldata[0..6],
            vec![
                Felt::ONE,
                *KAKAROT_ADDRESS,
                *ETH_SEND_TRANSACTION,
                Felt::ZERO,
                Felt::from((transaction.transaction.length() + 30) / 31 + 1),
                Felt::from((transaction.transaction.length() + 30) / 31 + 1),
            ]
        );
    }

    #[test]
    #[should_panic(expected = "CalldataExceededLimit(22500, 30018)")]
    fn test_transaction_data_to_starknet_calldata_too_large_calldata() {
        // Test that an example create transaction from goerli decodes properly
        let tx_bytes = hex!("b901f202f901ee05228459682f008459682f11830209bf8080b90195608060405234801561001057600080fd5b50610175806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c80630c49c36c14610030575b600080fd5b61003861004e565b604051610045919061011d565b60405180910390f35b60606020600052600f6020527f68656c6c6f2073746174656d696e64000000000000000000000000000000000060405260406000f35b600081519050919050565b600082825260208201905092915050565b60005b838110156100be5780820151818401526020810190506100a3565b838111156100cd576000848401525b50505050565b6000601f19601f8301169050919050565b60006100ef82610084565b6100f9818561008f565b93506101098185602086016100a0565b610112816100d3565b840191505092915050565b6000602082019050818103600083015261013781846100e4565b90509291505056fea264697066735822122051449585839a4ea5ac23cae4552ef8a96b64ff59d0668f76bfac3796b2bdbb3664736f6c63430008090033c080a0136ebffaa8fc8b9fda9124de9ccb0b1f64e90fbd44251b4c4ac2501e60b104f9a07eb2999eec6d185ef57e91ed099afb0a926c5b536f0155dd67e537c7476e1471");

        // Create a large tx_bytes by repeating the original tx_bytes 31 times
        let mut large_tx_bytes = Vec::new();
        for _ in 0..31 {
            large_tx_bytes.extend_from_slice(&tx_bytes);
        }

        // Decode the transaction from the provided bytes
        let mut transaction = TransactionSigned::decode(&mut &large_tx_bytes[..]).unwrap();

        // Set the input of the transaction to be a vector of 30,000 zero bytes
        transaction.transaction.set_input(vec![0; 30000 * 31].into());

        // Attempt to convert the transaction into a Starknet transaction
        transaction_data_to_starknet_calldata(&transaction, Felt::ZERO).unwrap();
    }
}
