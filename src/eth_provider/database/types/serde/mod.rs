use std::collections::HashMap;

use serde::{de::value::MapDeserializer, Deserialize, Deserializer};
use serde_json::Value;

/// Used in order to perform a custom deserialization of the stored
/// Ethereum data from the database. All the primitive types are stored
/// as strings in the database. This caused problems when deserializing.
///
/// # Example
///
/// The database stores {"hash": "0x1234"}. This gets serialized to
/// "{\"hash\":\"0x1234\"}". It's not possible to deserialize \"0x1234\"
/// into a U64 or B256 type from reth_primitives (since \"0x1234\" is the
/// serialized representation of the string "0x1234"). This function provides
/// a custom deserialization that first deserializes the data into a
/// HashMap<String, Value>, which can them be used to deserialize into the
/// desired types.
pub fn deserialize_intermediate<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let s: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
    let deserializer = MapDeserializer::new(s.into_iter());
    T::deserialize(deserializer).map_err(|err: serde_json::Error| serde::de::Error::custom(err.to_string()))
}
