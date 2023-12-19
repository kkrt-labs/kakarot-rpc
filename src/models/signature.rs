use reth_primitives::U256;
use reth_rpc_types::{Parity, Signature as EthSignature};
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
        let r_low: U256 = Felt252Wrapper::from(
            *value.0.get(0).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("r".to_string()))?,
        )
        .into();
        let r_high: U256 = Felt252Wrapper::from(
            *value.0.get(1).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("r".to_string()))?,
        )
        .into();
        let r = r_low + (r_high << 128);
        let s_low: U256 = Felt252Wrapper::from(
            *value.0.get(2).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("r".to_string()))?,
        )
        .into();
        let s_high: U256 = Felt252Wrapper::from(
            *value.0.get(3).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("r".to_string()))?,
        )
        .into();
        let s = s_low + (s_high << 128);
        let v: U256 = Felt252Wrapper::from(
            *value.0.get(4).ok_or_else(|| StarknetSignatureError::MissingSignatureParamsError("v".to_string()))?,
        )
        .into();
        let y_parity = if v == U256::from(0u8) {
            Some(Parity(false))
        } else if v == U256::from(1u8) {
            Some(Parity(true))
        } else {
            None
        };
        Ok(Self { r, s, v, y_parity })
    }
}

#[cfg(test)]
mod tests {
    use starknet::core::crypto::pedersen_hash;
    use starknet_crypto::{sign, ExtendedSignature};

    use crate::starknet_client::helpers::split_u256_into_field_elements;

    use super::*;

    pub const PRIVATE_KEY: &str = "0x0684e179baf957906d4a0e33bd28066778659964f6b5477e2483b72419a6b874";

    fn get_signature() -> (Vec<FieldElement>, ExtendedSignature) {
        let tx_hash = pedersen_hash(&FieldElement::from(1u8), &FieldElement::from(2u8));
        let private_key = FieldElement::from_hex_be(PRIVATE_KEY).unwrap();

        let signature = sign(&private_key, &tx_hash, &FieldElement::from(1u8)).unwrap();
        let r = Felt252Wrapper::from(signature.r);
        let r: U256 = r.into();
        let [r_low, r_high] = split_u256_into_field_elements(r);

        let s = Felt252Wrapper::from(signature.s);
        let s: U256 = s.into();
        let [s_low, s_high] = split_u256_into_field_elements(s);

        (vec![r_low, r_high, s_low, s_high, signature.v], signature)
    }

    #[test]
    fn test_starknet_to_eth_signature_passes() {
        let (starknet_signature, raw_signature) = get_signature();

        let eth_signature = EthSignature::try_from(StarknetSignature::from(starknet_signature)).unwrap();

        let y_parity = if raw_signature.v == FieldElement::ONE || raw_signature.v == FieldElement::ZERO {
            Some(Parity(raw_signature.v == FieldElement::ONE))
        } else {
            None
        };

        let expected_signature = EthSignature {
            r: Felt252Wrapper::from(raw_signature.r).into(),
            s: Felt252Wrapper::from(raw_signature.s).into(),
            v: Felt252Wrapper::from(raw_signature.v).into(),
            y_parity,
        };

        assert_eq!(eth_signature, expected_signature);
    }
}
