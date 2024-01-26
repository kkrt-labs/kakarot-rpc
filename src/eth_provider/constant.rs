use lazy_static::lazy_static;

lazy_static! {
    pub static ref MAX_FEE: u64 = 100_000_000_000_000_000u64;
    pub static ref BASE_FEE_PER_GAS: u64 = 1;
    pub static ref MAX_PRIORITY_FEE_PER_GAS: u64 = 0;
}
