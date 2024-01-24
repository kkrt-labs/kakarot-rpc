use jsonrpsee::types::ErrorObject;
use thiserror::Error;

use crate::starknet_client::errors::{rpc_err, EthRpcErrorCode};

/// Error that can occur when interacting with the database.
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// MongoDB error.
    #[error(transparent)]
    MongoDbError(#[from] mongodb::error::Error),
    /// Value not found in the database.
    #[error("Did not find value in the database.")]
    ValueNotFound,
}

impl From<DatabaseError> for jsonrpsee::core::Error {
    fn from(err: DatabaseError) -> Self {
        Self::Call(err.into())
    }
}

impl From<DatabaseError> for ErrorObject<'static> {
    fn from(value: DatabaseError) -> Self {
        rpc_err(EthRpcErrorCode::InternalError, value.to_string())
    }
}
