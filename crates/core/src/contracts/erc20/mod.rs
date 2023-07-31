pub mod ethereum_erc20;
pub mod starknet_erc20;

use ethers::abi::Error as AbiError;
use thiserror::Error;

#[derive(Debug, Error)]
/// Contract Error
pub enum ContractError {
    /// Contract Abi error
    #[error(transparent)]
    AbiError(#[from] AbiError),
}
