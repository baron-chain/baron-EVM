use bcevm::primitives::Address;
use serde::{de, Deserialize};

pub fn deserialize_str_as_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: de::Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .strip_prefix("0x")
        .map_or_else(
            |s| s.parse(),
            |s| u64::from_str_radix(s, 16)
        )
        .map_err(de::Error::custom)
}

pub fn deserialize_maybe_empty<'de, D>(deserializer: D) -> Result<Option<Address>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        s.parse().map(Some).map_err(de::Error::custom)
    }
}
