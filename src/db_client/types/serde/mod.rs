use std::str::FromStr;

use reth_primitives::{H256, U128, U256, U64};
use serde::{Deserialize as _, Deserializer};

macro_rules! deserialize_uint {
    ($name: ident, $type: ty, $pad: expr) => {
        pub fn $name<'de, D>(deserializer: D) -> Result<$type, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            // check if hex string
            if s.len() > 2 && &s[..2] == "0x" {
                <$type>::from_str_radix(&s[2..], 16).map_err(serde::de::Error::custom)
            } else {
                <$type>::from_str_radix(&s, 10).map_err(serde::de::Error::custom)
            }
        }
    };
}

deserialize_uint!(deserialize_u64, U64, 8);
deserialize_uint!(deserialize_u128, U128, 16);
deserialize_uint!(deserialize_u256, U256, 32);

pub fn deserialize_h256<'de, D>(deserializer: D) -> Result<H256, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    // pad with zeros if needed
    let hash = format!("0x{:0>64}", &s[2..]);
    H256::from_str(&hash).map_err(serde::de::Error::custom)
}
