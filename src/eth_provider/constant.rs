use lazy_static::lazy_static;

lazy_static! {
    pub static ref MAX_FEE: u64 = 100_000_000_000_000_000u64;
    /// Since Kakarot does not have a fee market, the base fee
    /// per gas should currently be the Starknet gas price.
    /// Since this field is not present in the Starknet
    /// block header, we arbitrarily set it to 100 Gwei.
    pub static ref BASE_FEE_PER_GAS: u64 = 100_000_000_000;
    pub static ref MAX_PRIORITY_FEE_PER_GAS: u64 = 0;
}
