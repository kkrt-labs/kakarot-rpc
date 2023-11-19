use std::slice::SliceIndex;

use reth_primitives::TransactionSigned;
use reth_rlp::Decodable;
use starknet::accounts::Call as StarknetCall;
use starknet_crypto::FieldElement;

use super::ConversionError;
use crate::models::DataDecodingError;

#[derive(Clone)]
pub struct Call(StarknetCall);

impl From<Call> for StarknetCall {
    fn from(call: Call) -> Self {
        call.0
    }
}

impl From<StarknetCall> for Call {
    fn from(call: StarknetCall) -> Self {
        Self(call)
    }
}

impl From<Call> for Vec<FieldElement> {
    fn from(call: Call) -> Self {
        let mut c = vec![
            FieldElement::ONE,
            call.0.to,
            call.0.selector,
            FieldElement::ZERO,
            FieldElement::from(call.0.calldata.len()),
            FieldElement::from(call.0.calldata.len()),
        ];
        c.extend(call.0.calldata);
        c
    }
}

pub struct Calls(Vec<StarknetCall>);

impl From<Calls> for Vec<StarknetCall> {
    fn from(calls: Calls) -> Self {
        calls.0
    }
}

impl From<Vec<StarknetCall>> for Calls {
    fn from(calls: Vec<StarknetCall>) -> Self {
        Self(calls)
    }
}

/// Converts a raw starknet transaction calldata to a vector of starknet calls.
impl TryFrom<Vec<FieldElement>> for Calls {
    type Error = ConversionError<()>;

    fn try_from(value: Vec<FieldElement>) -> Result<Self, Self::Error> {
        // in account calls, the calldata is first each call as {contract address, selector, data offset,
        // data length} and then all the calldata of each call, so each call takes 4 felts, and
        // eventually the calldata of the first call is at offset =  1 (for call_len) + 4 * call_len + 1
        // (for calldata_len)
        let calls_len = u32::try_from(value[0])
            .map_err(|e| ConversionError::ValueOutOfRange(format!("{}: call array length > u32::MAX", e)))?
            as usize;

        let mut offset = calls_len * 4 + 2;

        let mut calls = vec![];
        for i in 0..calls_len {
            let calldata_len =
                u32::try_from(value[i * 4 + 4]).map_err(|e| ConversionError::ValueOutOfRange(e.to_string()))? as usize;
            let call = StarknetCall {
                to: value[i * 4 + 2],
                selector: value[i * 4 + 3],
                calldata: value[offset..offset + calldata_len].to_vec(),
            };
            offset += calldata_len;
            calls.push(call);
        }
        Ok(Self(calls))
    }
}

impl TryFrom<&Calls> for TransactionSigned {
    type Error = DataDecodingError;

    fn try_from(value: &Calls) -> std::result::Result<Self, Self::Error> {
        if value.len() > 1 {
            return Err(DataDecodingError::SignatureDecodingError(
                "call array length > 1 is not supported".to_string(),
            ));
        }

        let call = value.0[0] // for now we decode signature only from the first call
            .calldata
            .iter()
            .filter_map(|x| u8::try_from(*x).ok())
            .collect::<Vec<u8>>();
        Self::decode(&mut call.as_slice()).map_err(|e| DataDecodingError::SignatureDecodingError(e.to_string()))
    }
}

impl Calls {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[StarknetCall]>,
    {
        self.0.get(index)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reth_primitives::{Address, U256};
    use serde::Deserialize;

    use super::*;
    use crate::client::constants::selectors::ETH_CALL;
    use crate::mock::constants::ACCOUNT_ADDRESS;

    #[derive(Deserialize)]
    struct SerdeCall {
        pub calldata: Vec<FieldElement>,
    }

    #[test]
    fn test_from_call() {
        // Given
        let call: Call = StarknetCall {
            to: *ACCOUNT_ADDRESS,
            selector: ETH_CALL,
            calldata: vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8].into_iter().map(FieldElement::from).collect(),
        }
        .into();

        // When
        let raw_calldata = Vec::<FieldElement>::from(call);

        // Then
        let expected = vec![
            FieldElement::ONE,
            *ACCOUNT_ADDRESS,
            ETH_CALL,
            FieldElement::ZERO,
            FieldElement::from(6u8),
            FieldElement::from(6u8),
            FieldElement::from(1u8),
            FieldElement::from(2u8),
            FieldElement::from(3u8),
            FieldElement::from(4u8),
            FieldElement::from(5u8),
            FieldElement::from(6u8),
        ];
        assert_eq!(expected, raw_calldata);
    }

    #[test]
    fn test_try_from_calls() {
        // Given
        let raw: Vec<FieldElement> = serde_json::from_str(include_str!("test_data/call/raw_call.json")).unwrap();

        // When
        let calls: Calls = raw.try_into().unwrap();

        // Then
        let expected: Vec<SerdeCall> = serde_json::from_str(include_str!("test_data/call/call.json")).unwrap();
        assert_eq!(3, calls.len());
        assert_eq!(expected[0].calldata, calls.get(0).unwrap().calldata);
        assert_eq!(expected[1].calldata, calls.get(1).unwrap().calldata);
        assert_eq!(expected[2].calldata, calls.get(2).unwrap().calldata);
    }

    #[test]
    fn test_calls_get_signature_should_pass() {
        // Given
        let raw: Vec<FieldElement> = serde_json::from_str(include_str!("test_data/call/kakarot_call.json")).unwrap();
        let calls: Calls = raw.try_into().unwrap();

        // When
        let signature = TryInto::<TransactionSigned>::try_into(&calls).unwrap().signature;

        // Then
        assert_eq!(
            signature.r,
            U256::from_str("0x889be67d59bc1a43dd803955f7917ddcb7d748ed3e9b00cdb159f294651976b8").unwrap()
        );
        assert_eq!(
            signature.s,
            U256::from_str("0x03801702a606ffbfd60364ff897f7ca511411d6660f936dd51eb90a7d30735261").unwrap()
        );
        assert!(signature.odd_y_parity);
    }

    #[test]
    #[should_panic(expected = "SignatureDecodingError(\"call array length > 1 is not supported\")")]
    fn test_calls_get_signature_should_fail() {
        // Given
        let raw: Vec<FieldElement> = serde_json::from_str(include_str!("test_data/call/raw_call.json")).unwrap();
        let calls: Calls = raw.try_into().unwrap();

        // When
        TryInto::<TransactionSigned>::try_into(&calls).unwrap();
    }

    #[test]
    fn test_calls_get_to() {
        // Given
        let raw: Vec<FieldElement> = serde_json::from_str(include_str!("test_data/call/kakarot_call.json")).unwrap();
        let calls: Calls = raw.try_into().unwrap();

        // When
        let to = TryInto::<TransactionSigned>::try_into(&calls).unwrap().to();

        // Then
        assert_eq!(to, Some(Address::from_str("0x2e11ed82f5ec165ab8ce3cc094f025fe7527f4d1").unwrap()));
    }

    #[test]
    fn test_slice_calls() {
        // Given
        let raw: Vec<FieldElement> = serde_json::from_str(include_str!("test_data/call/raw_call.json")).unwrap();
        let calls: Calls = raw.try_into().unwrap();

        // When
        let slice = calls.get(1..3).unwrap();

        // Then
        let expected: Vec<SerdeCall> = serde_json::from_str(include_str!("test_data/call/call.json")).unwrap();
        assert_eq!(slice.len(), 2);
        assert_eq!(expected[1].calldata, slice[0].calldata);
        assert_eq!(expected[2].calldata, slice[1].calldata)
    }
}
