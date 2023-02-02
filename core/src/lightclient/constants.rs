// constants.rs
pub const ACCOUNT_REGISTRY_ADDRESS: &str =
    "0x702ff500f359a185fafbdef2fadad75b04e21a814abc5e6257e2e65ceffdf15";
pub const KAKAROT_MAIN_CONTRACT_ADDRESS: &str =
    "0xb5644ba96891f73151973d76f914ee6eea75479a1fe5fbe0098afa50027385";
pub const CHAIN_ID: u64 = 1263227476;

pub mod selectors {
    use starknet::core::types::FieldElement;
    use starknet::macros::selector;

    pub const GET_STARKNET_CONTRACT_ADDRESS: FieldElement =
        selector!("get_starknet_contract_address");
    pub const BYTECODE: FieldElement = selector!("bytecode");

    pub const EXECUTE_AT_ADDRESS: FieldElement = selector!("execute_at_address");
}
