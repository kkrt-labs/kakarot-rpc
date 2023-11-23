use reth_rpc_types::Signature as EthSignature;
use starknet::core::types::FieldElement;
use thiserror::Error;

use super::felt::Felt252Wrapper;

#[derive(Debug, Error, PartialEq, Eq)]
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

impl TryFrom<StarknetSignature> for EthSignature {
    type Error = StarknetSignatureError;

    fn try_from(value: StarknetSignature) -> Result<Self, Self::Error> {
        let r: Felt252Wrapper =
            (*value.0.get(0).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("r".to_string()))?)
                .into();
        let s: Felt252Wrapper =
            (*value.0.get(1).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("s".to_string()))?)
                .into();
        let v: Felt252Wrapper =
            (*value.0.get(2).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("v".to_string()))?)
                .into();
        Ok(Self { r: r.into(), s: s.into(), v: v.into(), y_parity: None })
    }
}

#[cfg(test)]
mod tests {
    use starknet::core::crypto::pedersen_hash;
    use starknet_crypto::{sign, ExtendedSignature};

    use super::*;

    pub const PRIVATE_KEY: &str = "0x0684e179baf957906d4a0e33bd28066778659964f6b5477e2483b72419a6b874";

    fn get_signature() -> ExtendedSignature {
        let tx_hash = pedersen_hash(&FieldElement::from(1u8), &FieldElement::from(2u8));
        let private_key = FieldElement::from_hex_be(PRIVATE_KEY).unwrap();

        sign(&private_key, &tx_hash, &FieldElement::from(1u8)).unwrap()
    }

    #[test]
    fn test_starknet_to_eth_signature_passes() {
        let starknet_signature = get_signature();
        let flattened_signature = vec![starknet_signature.r, starknet_signature.s, starknet_signature.v];

        let eth_signature = EthSignature::try_from(StarknetSignature::from(flattened_signature)).unwrap();

        let r: Felt252Wrapper = starknet_signature.r.into();
        assert_eq!(eth_signature.r, r.into());

        let s: Felt252Wrapper = starknet_signature.s.into();
        assert_eq!(eth_signature.s, s.into());

        let v: Felt252Wrapper = starknet_signature.v.into();
        assert_eq!(eth_signature.v, v.into());
    }

    #[test]
    fn test_starknet_to_eth_signature_fails_on_missing_signature_params() {
        let starknet_signature = get_signature();
        let flattened_signature = vec![starknet_signature.r, starknet_signature.s];

        assert_eq!(
            StarknetSignatureError::MissingSignatureParamsError("v".to_string()),
            EthSignature::try_from(StarknetSignature::from(flattened_signature)).unwrap_err()
        );
    }
}
