#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate starknet;

use starknet::{

    core::{ types::{FieldElement,BlockId,CallFunction}},
    providers::{SequencerGatewayProvider, Provider},
    macros::{felt, selector},

};

// Good First RPCs
//eth_blockNumber
//eth_getCode
//eth_getTransactionByHash
//eth_sendTransaction

#[get("/execute/<tx_bytecode>/<tx_calldata>")]
async fn execute(   
    tx_bytecode: String, 
    tx_calldata: String) -> String {
    println!("Transaction Calldata: {}",&tx_calldata);
    call_execute(tx_bytecode,tx_calldata).await
}

// Call Execute function on Kakarot contract
async fn call_execute(bytecode: String, calldata: String) -> String{

    // Get the provider for goerli 2
    let provider = SequencerGatewayProvider::starknet_alpha_goerli_2();

    // Kakarot Contract Address on Goerli 2
    let kakarot_token_address =
        felt!("0x031ddf73d0285cc2f08bd4a2c93229f595f2f6e64b25846fc0957a2faa7ef7bb");

    // Value for the transaction
    let value = FieldElement::from_dec_str("00").unwrap(); 

    // Apply the FieldElement::from_hex_be method to each hexadecimal string in the Vec
    let bytecode_vec: Vec<FieldElement> = hex_string_to_felt_vec(bytecode);
    let calldata_vec: Vec<FieldElement> = hex_string_to_felt_vec(calldata);

    // Get bytecode and calldata length
    let bytecode_len = FieldElement::from_dec_str(bytecode_vec.len().to_string().as_str()).unwrap();
    let calldata_len =FieldElement::from_dec_str(calldata_vec.len().to_string().as_str()).unwrap();

    // Create the calldata vector
    let mut tx_calldata_vec = vec![value];
    tx_calldata_vec.push(bytecode_len);
    tx_calldata_vec.extend(bytecode_vec);
    tx_calldata_vec.push(calldata_len);
    tx_calldata_vec.extend(calldata_vec);

    // Call Read Execute in Kakarot contract
    let call_result = provider
        .call_contract(
            CallFunction {
                contract_address: kakarot_token_address,
                entry_point_selector: selector!("execute"),
                calldata: tx_calldata_vec,
            },
            BlockId::Latest,
        )
        .await
        .expect("failed to call contract");

    // Return the result of the call in String
    format!("{:?}", call_result)
}

// Convert a hexadecimal string to a vector of FieldElements
fn hex_string_to_felt_vec(hex_string:String) -> Vec<FieldElement> {
    // Split the String into groups of two characters
    let hex_strings: Vec<String> = hex_string.chars().collect::<Vec<char>>()
        .chunks(2)
        .map(|chunk| {
            // Concatenate the characters into a single String
            chunk.iter().collect::<String>()
        })
        .collect();
    
    // Apply the Field::from_hex_be method to each hexadecimal string in the Vec
    let felt_vec: Vec<FieldElement> = hex_strings
        .into_iter()
        .map(|hex_string| FieldElement::from_hex_be(&hex_string).unwrap())
        .collect();        

    felt_vec   
}


#[launch]

fn rocket() -> _ {
    rocket::build().mount("/kakarot/", routes![execute])
}