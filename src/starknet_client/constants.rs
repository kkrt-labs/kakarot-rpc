use lazy_static::lazy_static;
use reth_primitives::{Address, H256, H64, U128, U256, U8};
use starknet::accounts::Call as StarknetCall;
use starknet::core::types::FieldElement;
use starknet::macros::selector;

use crate::models::call::Call;

pub const STARKNET_NATIVE_TOKEN: &str = "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";

pub const EARLIEST_BLOCK_NUMBER: u64 = 0;

/// Current chunk limit for pathfinder https://github.com/eqlabs/pathfinder/blob/main/crates/storage/src/connection/event.rs#L11
pub const CHUNK_SIZE_LIMIT: u64 = 1024;

pub const MADARA_RPC_URL: &str = "http://127.0.0.1:9944";

pub const KATANA_RPC_URL: &str = "http://0.0.0.0:5050";

pub mod selectors {
    use starknet::core::types::FieldElement;
    use starknet::macros::selector;

    pub const BYTECODE: FieldElement = selector!("bytecode");
    pub const STORAGE: FieldElement = selector!("storage");
    pub const GET_IMPLEMENTATION: FieldElement = selector!("get_implementation");
    pub const GET_NONCE: FieldElement = selector!("get_nonce");

    pub const ETH_CALL: FieldElement = selector!("eth_call");
    pub const ETH_SEND_TRANSACTION: FieldElement = selector!("eth_send_transaction");
    pub const DEPLOY_EXTERNALLY_OWNED_ACCOUNT: FieldElement = selector!("deploy_externally_owned_account");
    pub const COMPUTE_STARKNET_ADDRESS: FieldElement = selector!("compute_starknet_address");

    pub const GET_EVM_ADDRESS: FieldElement = selector!("get_evm_address");

    pub const BALANCE_OF: FieldElement = selector!("balanceOf");

    pub const EVM_CONTRACT_DEPLOYED: FieldElement = selector!("evm_contract_deployed");
}

/// This module contains constants related to EVM gas fees.
pub mod gas {
    use reth_primitives::U128;

    /// The base fee for a transaction in gwei.
    ///
    /// Since Starknet does not currently have a market for gas fees
    /// TODO: Get Starknet "historical" Gas Price instead
    pub const BASE_FEE_PER_GAS: u64 = 1;

    /// The maximum priority fee for a transaction in gwei.
    ///
    /// This fee is the maximum amount a user is willing to pay to have their transaction
    /// included in a block quickly.
    ///
    /// Since Starknet does not currently have a market for gas fees, transactions are processed
    /// on a "first come first served" basis by the Sequencer.
    /// As a result, the priority fee is set to 0.
    pub const MAX_PRIORITY_FEE_PER_GAS: U128 = U128::ZERO;

    /// The minimum gas fee for a transaction
    ///
    /// This minimum of 21,000 (see https://ethereum.stackexchange.com/questions/34674/where-does-the-number-21000-come-from-for-the-base-gas-consumption-in-ethereum/34675#34675)
    /// is used if the returned fee estimate is lower, otherwise wallets such as Metamask will not
    /// allow the transaction to be sent.
    pub const MINIMUM_GAS_FEE: u64 = 21_000;
}

/// This module contains error messages related to Kakarot.
pub mod error_messages {
    /// Error message when a transaction is not part of Kakarot.
    pub const INVALID_TRANSACTION_TYPE: &str = "L1Handler, Declare, Deploy and DeployAccount transactions unsupported";
}

// This module contains constants which are being used in place of real data that should be fetched
// in production.
lazy_static! {
    pub static ref GAS_LIMIT: U256 = U256::from(1_000_000u64);
    pub static ref GAS_USED: U256 = U256::from(500_000u64);
    pub static ref CUMULATIVE_GAS_USED: U256 = U256::from(1_000_000u64);
    pub static ref EFFECTIVE_GAS_PRICE: U128 = U128::from(1_000_000u64);
    pub static ref SIZE: Option<U256> = Some(U256::from(1_000_000u64));
    pub static ref MAX_FEE: FieldElement = FieldElement::from(100_000_000_000_000_000u64);
    pub static ref ESTIMATE_GAS: U256 = U256::from(100_000_000_000_000_000u64);
    pub static ref TRANSACTION_TYPE: U8 = U8::from(0);
    pub static ref NONCE: Option<H64> = Some(H64::zero());
    pub static ref MIX_HASH: H256 = H256::zero();
    pub static ref DIFFICULTY: U256 = U256::from(0);
    pub static ref TOTAL_DIFFICULTY: Option<U256> = None;
    pub static ref TX_ORIGIN_ZERO: Address = Address::zero();
}

lazy_static! {
    pub static ref KAKAROT_CLIENT_VERSION: String = format!("kakarot_{}", env!("CARGO_PKG_VERSION"));
}

// This module contains constants related to transactions used to calculate
// Starknet gas price
lazy_static! {
    /// The address of the argent account used to calculate the gas price.
    /// (code: https://github.com/argentlabs/argent-contracts-starknet/blob/develop/contracts/account/src/argent_account.cairo)
    /// This account is ONLY used for the gasPrice JSON RPC route, to send a simulate_transaction payload for a dummy transaction on Starknet
    /// Thus recovering the current gas_price
    pub static ref DUMMY_ARGENT_GAS_PRICE_ACCOUNT_ADDRESS: FieldElement =
        FieldElement::from_hex_be("0x07142FbF6E8C9C07b079D47727C6D2ff49970203bfd5Bd6ED0D740e0f5a344E7").unwrap();
    pub static ref INC_SELECTOR: FieldElement = selector!("inc");
    /// The address of the counter contract used to calculate the gas price on mainnet
    /// (code: https://gist.github.com/greged93/78b58f85cba6cf76eefaedab87f1b645)
    pub static ref COUNTER_ADDRESS_MAINNET: FieldElement =
        FieldElement::from_hex_be("0x02786c4cdfb2ee39727cb00695cf136710e2c3bfc5cb09315101be3d37c2c557").unwrap();
    /// The address of the counter contract used to calculate the gas price on goerli 1
    pub static ref COUNTER_ADDRESS_TESTNET1: FieldElement =
        FieldElement::from_hex_be("0x03c12643f0e9f0b41de95a87e4f03f5fa69601930e9354a206a0b82a02119f2b").unwrap();
    /// The address of the counter contract used to calculate the gas price on goerli 2
    pub static ref COUNTER_ADDRESS_TESTNET2: FieldElement =
        FieldElement::from_hex_be("0x00e438661a4775fdf10cf132cc50730f40e59f3d040b15e64cd292add25eb01b").unwrap();
    pub static ref COUNTER_CALL_MAINNET: Call =
        StarknetCall { to: *COUNTER_ADDRESS_MAINNET, selector: *INC_SELECTOR, calldata: vec![] }.into();
    pub static ref COUNTER_CALL_TESTNET1: Call =
        StarknetCall { to: *COUNTER_ADDRESS_TESTNET1, selector: *INC_SELECTOR, calldata: vec![] }.into();
    pub static ref COUNTER_CALL_TESTNET2: Call =
        StarknetCall { to: *COUNTER_ADDRESS_TESTNET2, selector: *INC_SELECTOR, calldata: vec![] }.into();
}

// This module contains constants to be used for deployment of Kakarot System
lazy_static! {
    pub static ref DEPLOY_FEE: FieldElement = FieldElement::from(100_000_u64);
}
