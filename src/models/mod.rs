pub mod balance;
pub mod block;
pub mod felt;
pub mod transaction;

#[derive(Debug, thiserror::Error)]
#[error("conversion failed")]
pub struct ConversionError;
