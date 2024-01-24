use std::collections::HashMap;

use serde::{de::value::MapDeserializer, Deserialize, Deserializer};
use serde_json::Value;

pub fn deserialize_intermediate<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // Deserialize the map into a `HashMap<String, Value>`. This intermediate
    // step is required because the primitives types of the transaction (e.g. U64,
    // H256) are stored as strings in the database. This caused problems when
    // deserializing the transaction because the deserializer expects a serialized
    // primitives, not a serialized string.
    let s: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
    let deserializer = MapDeserializer::new(s.into_iter());
    T::deserialize(deserializer).map_err(|err: serde_json::Error| serde::de::Error::custom(err.to_string()))
}
