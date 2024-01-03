use std::slice::SliceIndex;

use bytes::Buf;
use reth_primitives::{Bytes, Transaction, TxEip1559, TxEip2930, TxLegacy};
use reth_rlp::{Decodable, Header};
use starknet::accounts::Call as StarknetCall;
use starknet_crypto::FieldElement;

use crate::models::errors::ConversionError;
use crate::starknet_client::helpers::DataDecodingError;

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
    type Error = ConversionError;

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

impl TryFrom<Call> for Transaction {
    type Error = DataDecodingError;

    fn try_from(value: Call) -> std::result::Result<Self, Self::Error> {
        let call = value.0.calldata.into_iter().filter_map(|x| u8::try_from(x).ok()).collect::<Vec<u8>>();

        let maybe_tx_type = call
            .first()
            .ok_or(DataDecodingError::CalldataDecodingError("Transaction Calldata is empty".to_string()))?;

        // If tx_type is superior to 0xb7, it's a legacy tx, otherwise we match on tx type 1, 2 or else
        // Logic and code snippets taken and adapted from reth_primitives and reth_rlp crates
        let mut buf = call.as_slice();
        if maybe_tx_type > &0xbf {
            let _ = Header::decode(&mut buf)?;
            let mut tx = TxLegacy {
                nonce: Decodable::decode(&mut buf)?,
                gas_price: Decodable::decode(&mut buf)?,
                gas_limit: Decodable::decode(&mut buf)?,
                to: Decodable::decode(&mut buf)?,
                value: Decodable::decode(&mut buf)?,
                input: Bytes(Decodable::decode(&mut buf)?),
                chain_id: None,
            };
            // Extract the chain_id separately as the TxLegacy struct expects an Option
            let chain_id = Decodable::decode(&mut buf)?;
            tx.chain_id = Some(chain_id);
            return Ok(Transaction::Legacy(tx));
        }
        // Skip the transaction type
        let _ = &mut buf.advance(1);
        // decode the list header for the rest of the transaction
        let header = Header::decode(&mut buf)?;
        if !header.list {
            return Err(DataDecodingError::CalldataDecodingError(
                "Typed tx fields must be encoded as a list".to_string(),
            ));
        }
        match maybe_tx_type {
            1 => Ok(Transaction::Eip2930(TxEip2930 {
                chain_id: Decodable::decode(&mut buf)?,
                nonce: Decodable::decode(&mut buf)?,
                gas_price: Decodable::decode(&mut buf)?,
                gas_limit: Decodable::decode(&mut buf)?,
                to: Decodable::decode(&mut buf)?,
                value: Decodable::decode(&mut buf)?,
                input: Bytes(Decodable::decode(&mut buf)?),
                access_list: Decodable::decode(&mut buf)?,
            })),
            2 => Ok(Transaction::Eip1559(TxEip1559 {
                chain_id: Decodable::decode(&mut buf)?,
                nonce: Decodable::decode(&mut buf)?,
                max_priority_fee_per_gas: Decodable::decode(&mut buf)?,
                max_fee_per_gas: Decodable::decode(&mut buf)?,
                gas_limit: Decodable::decode(&mut buf)?,
                to: Decodable::decode(&mut buf)?,
                value: Decodable::decode(&mut buf)?,
                input: Bytes(Decodable::decode(&mut buf)?),
                access_list: Decodable::decode(&mut buf)?,
            })),
            _ => Err(DataDecodingError::CalldataDecodingError("Transaction type is not supported".to_string())),
        }
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

    use reth_primitives::Address;
    use serde::Deserialize;
    use starknet::macros::felt;

    use super::*;
    use crate::starknet_client::constants::selectors::ETH_CALL;

    #[derive(Debug, Deserialize)]
    pub struct TestCall {
        pub to: FieldElement,
        pub selector: FieldElement,
        pub calldata: Vec<FieldElement>,
    }

    // Impl From for TestCall into StarknetCall
    impl From<TestCall> for StarknetCall {
        fn from(call: TestCall) -> Self {
            Self { to: call.to, selector: call.selector, calldata: call.calldata }
        }
    }

    #[test]
    fn test_from_call() {
        // Given
        let call: Call = StarknetCall {
            to: felt!("0xdead"),
            selector: ETH_CALL,
            calldata: vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8].into_iter().map(FieldElement::from).collect(),
        }
        .into();

        // When
        let raw_calldata = Vec::<FieldElement>::from(call);

        // Then
        let expected = vec![
            FieldElement::ONE,
            felt!("0xdead"),
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
    fn test_calls_get_to_eip1559() {
        // Given
        let raw: TestCall = serde_json::from_str(include_str!("test_data/call/eip1559.json")).unwrap();
        let starknet_call: StarknetCall = raw.into();
        let call = Call::from(starknet_call);

        // When
        let to = TryInto::<Transaction>::try_into(call).unwrap().to();

        // Then
        assert_eq!(to, Some(Address::from_str("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984").unwrap()));
    }

    #[test]
    fn test_calls_get_to_eip2930() {
        // Given
        let raw: TestCall = serde_json::from_str(include_str!("test_data/call/eip2930.json")).unwrap();
        let starknet_call: StarknetCall = raw.into();
        let call = Call::from(starknet_call);

        // When
        let to = TryInto::<Transaction>::try_into(call).unwrap().to();

        // Then
        assert_eq!(to, Some(Address::from_str("0x0000006f746865725f65766d5f61646472657373").unwrap()));
    }

    #[test]
    fn test_calls_get_to_legacy() {
        // Given
        let raw: TestCall = serde_json::from_str(include_str!("test_data/call/legacy.json")).unwrap();
        let starknet_call: StarknetCall = raw.into();
        let call = Call::from(starknet_call);

        // When
        let to = TryInto::<Transaction>::try_into(call).unwrap().to();

        // Then
        assert_eq!(to, Some(Address::from_str("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984").unwrap()));
    }
}
