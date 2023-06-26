pub mod balance;
pub mod block;
pub mod convertible;
pub mod felt;
pub mod transaction;

use thiserror::Error;

use self::felt::Felt252WrapperError;
use crate::client::helpers::DataDecodingError;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("transaction conversion error: {0}")]
    TransactionConversionError(String),
    #[error(transparent)]
    Felt252WrapperConversionError(#[from] Felt252WrapperError),
    #[error(transparent)]
    DataDecodingError(#[from] DataDecodingError),
}
