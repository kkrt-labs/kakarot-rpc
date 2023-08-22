use std::str::FromStr;

use ethers::signers::LocalWallet;
use lazy_static::lazy_static;
use reth_primitives::Address;
use starknet_crypto::FieldElement;

pub const EOA_PRIVATE_KEY: &str = "024b7c9e8f15432309db022c54d3279d9b421275533e090aa03cbf4211670823";

pub const EVM_CONTRACTS: &[&str] = &["ERC20", "Counter", "PlainOpcodes"];

lazy_static! {
    pub static ref EOA_WALLET: LocalWallet = EOA_PRIVATE_KEY.parse().unwrap();
    pub static ref STARKNET_DEPLOYER_ACCOUNT_PRIVATE_KEY: FieldElement =
        FieldElement::from_hex_be("0x0288a51c164874bb6a1ca7bd1cb71823c234a86d0f7b150d70fa8f06de645396").unwrap();
    // the address has been taken from ganache -> https://github.com/trufflesuite/ganache
    pub static ref EOA_RECEIVER_ADDRESS: Address = Address::from_str("0xC9A2d92c5913eDEAd9a7C936C96631F0F2241063").unwrap();
}
