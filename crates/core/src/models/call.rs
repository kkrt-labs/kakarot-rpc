use reth_primitives::{Address, Bytes, Signature, TransactionSigned};
use reth_rlp::Decodable;
use starknet::accounts::Call;
use starknet_crypto::FieldElement;

use super::ConversionError;
use crate::models::DataDecodingError;

pub struct Calls(Vec<Call>);

impl From<Calls> for Vec<Call> {
    fn from(calls: Calls) -> Self {
        calls.0
    }
}

impl From<Vec<Call>> for Calls {
    fn from(calls: Vec<Call>) -> Self {
        Self(calls)
    }
}

/// TryFrom implementation for account contract calls
impl TryFrom<Vec<FieldElement>> for Calls {
    type Error = ConversionError;

    fn try_from(value: Vec<FieldElement>) -> Result<Self, Self::Error> {
        let calls_len = u32::try_from(value[0]).map_err(|e| ConversionError::ValueOutOfRange(e.to_string()))? as usize;
        let mut offset = calls_len * 4 + 2;

        let mut calls = vec![];
        for i in 0..calls_len {
            let calldata_len =
                u32::try_from(value[i * 4 + 4]).map_err(|e| ConversionError::ValueOutOfRange(e.to_string()))? as usize;
            let call = Call {
                to: value[i * 4 + 2],
                selector: value[i * 4 + 3],
                calldata: value[offset..offset + calldata_len].to_vec(),
            };
            offset += calldata_len;
            calls.push(call);
        }
        Ok(Calls(calls))
    }
}

impl TryFrom<&Calls> for TransactionSigned {
    type Error = DataDecodingError;

    fn try_from(value: &Calls) -> std::result::Result<Self, Self::Error> {
        let call = value.0[0] // for now we decode signature only from the first call
            .calldata
            .iter()
            .filter_map(|x| u8::try_from(*x).ok())
            .collect::<Vec<u8>>();
        TransactionSigned::decode(&mut call.as_slice())
            .map_err(|e| DataDecodingError::SignatureDecodingError(e.to_string()))
    }
}

impl Calls {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<Call> {
        self.0.get(index).cloned()
    }

    pub fn get_eth_transaction_input(&self) -> Result<Bytes, DataDecodingError> {
        let tx: TransactionSigned = self.try_into()?;
        return Ok(tx.input().to_owned());
    }

    pub fn get_eth_transaction_to(&self) -> Result<Option<Address>, DataDecodingError> {
        let tx: TransactionSigned = self.try_into()?;
        Ok(tx.to())
    }

    pub fn get_eth_transaction_signature(&self) -> Result<Signature, DataDecodingError> {
        let tx: TransactionSigned = self.try_into()?;
        Ok(tx.signature)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reth_primitives::U256;

    use super::*;

    fn to_vec_field_element(vec: Vec<&str>) -> Vec<FieldElement> {
        vec.into_iter().filter_map(|f| FieldElement::from_hex_be(f).ok()).collect()
    }

    fn get_test_calldata() -> Vec<FieldElement> {
        let calldata = vec![
            "0x01",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00",
            "0x075",
            "0x075",
            "0x02",
            "0x0f8",
            "0x072",
            "0x084",
            "0x04b",
            "0x04b",
            "0x052",
            "0x054",
            "0x082",
            "0x0de",
            "0x0ad",
            "0x082",
            "0x0de",
            "0x0ad",
            "0x082",
            "0x0de",
            "0x0ad",
            "0x084",
            "0x03b",
            "0x09a",
            "0x0ca",
            "0x00",
            "0x094",
            "0x02e",
            "0x011",
            "0x0ed",
            "0x082",
            "0x0f5",
            "0x0ec",
            "0x016",
            "0x05a",
            "0x0b8",
            "0x0ce",
            "0x03c",
            "0x0c0",
            "0x094",
            "0x0f0",
            "0x025",
            "0x0fe",
            "0x075",
            "0x027",
            "0x0f4",
            "0x0d1",
            "0x080",
            "0x084",
            "0x0b3",
            "0x0bc",
            "0x0fa",
            "0x082",
            "0x0c0",
            "0x01",
            "0x0a0",
            "0x088",
            "0x09b",
            "0x0e6",
            "0x07d",
            "0x059",
            "0x0bc",
            "0x01a",
            "0x043",
            "0x0dd",
            "0x080",
            "0x039",
            "0x055",
            "0x0f7",
            "0x091",
            "0x07d",
            "0x0dc",
            "0x0b7",
            "0x0d7",
            "0x048",
            "0x0ed",
            "0x03e",
            "0x09b",
            "0x00",
            "0x0cd",
            "0x0b1",
            "0x059",
            "0x0f2",
            "0x094",
            "0x065",
            "0x019",
            "0x076",
            "0x0b8",
            "0x0a0",
            "0x038",
            "0x01",
            "0x070",
            "0x02a",
            "0x060",
            "0x06f",
            "0x0fb",
            "0x0fd",
            "0x060",
            "0x036",
            "0x04f",
            "0x0f8",
            "0x097",
            "0x0f7",
            "0x0ca",
            "0x051",
            "0x014",
            "0x011",
            "0x0d6",
            "0x066",
            "0x0f",
            "0x093",
            "0x06d",
            "0x0d5",
            "0x01e",
            "0x0b9",
            "0x0a",
            "0x07d",
            "0x030",
            "0x073",
            "0x052",
            "0x061",
        ];
        to_vec_field_element(calldata)
    }

    #[test]
    fn test_try_from_calls() {
        let calldata = vec![
            "0x03",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00",
            "0x00a",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00a",
            "0x005",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00f",
            "0x005",
            "0x014",
            "0x000",
            "0x001",
            "0x002",
            "0x003",
            "0x004",
            "0x005",
            "0x006",
            "0x007",
            "0x008",
            "0x009",
            "0x00a",
            "0x00b",
            "0x00c",
            "0x00d",
            "0x00e",
            "0x00f",
            "0x010",
            "0x011",
            "0x012",
            "0x013",
        ];
        let calldata = to_vec_field_element(calldata);
        let calls = Calls::try_from(calldata).unwrap();
        assert_eq!(calls.len(), 3);
        let calldata = to_vec_field_element(vec![
            "0x000", "0x001", "0x002", "0x003", "0x004", "0x005", "0x006", "0x007", "0x008", "0x009",
        ]);
        assert_eq!(calls.get(0).unwrap().calldata, calldata);
        let calldata = to_vec_field_element(vec!["0x00a", "0x00b", "0x00c", "0x00d", "0x00e"]);
        assert_eq!(calls.get(1).unwrap().calldata, calldata);
        let calldata = to_vec_field_element(vec!["0x00f", "0x010", "0x011", "0x012", "0x013"]);
        assert_eq!(calls.get(2).unwrap().calldata, calldata);
    }

    #[test]
    fn test_calls_get_signature() {
        let calls: Calls = get_test_calldata().try_into().unwrap();
        let signature = calls.get_eth_transaction_signature().unwrap();
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
    fn test_calls_get_to() {
        let calls: Calls = get_test_calldata().try_into().unwrap();
        let to = calls.get_eth_transaction_to().unwrap();
        assert_eq!(to, Some(Address::from_str("0x2e11ed82f5ec165ab8ce3cc094f025fe7527f4d1").unwrap()));
    }
}
