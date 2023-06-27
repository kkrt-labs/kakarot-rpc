use reth_rpc_types::Signature;
use starknet::core::types::FieldElement;
use thiserror::Error;

use super::felt::Felt252Wrapper;

#[derive(Debug, Error)]
pub enum StarknetSignatureError {
    #[error("missing Starknet signature param {0}")]
    MissingSignatureParamsError(String),
}

pub struct StarknetSignature(Vec<FieldElement>);

impl From<Vec<FieldElement>> for StarknetSignature {
    fn from(value: Vec<FieldElement>) -> Self {
        Self(value)
    }
}

impl TryFrom<StarknetSignature> for Signature {
    type Error = StarknetSignatureError;

    fn try_from(value: StarknetSignature) -> Result<Self, Self::Error> {
        let r: Felt252Wrapper =
            (*value.0.get(0).ok_or(StarknetSignatureError::MissingSignatureParamsError("r".to_string()))?).into();
        let s: Felt252Wrapper =
            (*value.0.get(1).ok_or(StarknetSignatureError::MissingSignatureParamsError("s".to_string()))?).into();
        let v: Felt252Wrapper =
            (*value.0.get(2).ok_or(StarknetSignatureError::MissingSignatureParamsError("v".to_string()))?).into();
        Ok(Signature { r: r.into(), s: s.into(), v: v.into() })
    }
}
