use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, DeserializeAs, SerializeAs};
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::JsonRpcMethod;

impl SerializeAs<FieldElement> for UfeHex {
    fn serialize_as<S>(value: &FieldElement, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{value:#x}"))
    }
}

impl<'de> DeserializeAs<'de, FieldElement> for UfeHex {
    fn deserialize_as<D>(deserializer: D) -> Result<FieldElement, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match FieldElement::from_hex_be(&value) {
            Ok(value) => Ok(value),
            Err(err) => Err(DeError::custom(format!("invalid hex string: {err}"))),
        }
    }
}

pub struct UfeHex;

#[serde_as]
#[derive(Serialize, Deserialize)]
struct Felt(#[serde_as(as = "UfeHex")] pub FieldElement);

#[derive(Debug, Serialize)]
struct JsonRpcRequest<T> {
    id: u64,
    jsonrpc: &'static str,
    method: JsonRpcMethod,
    params: T,
}
