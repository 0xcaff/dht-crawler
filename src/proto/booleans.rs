use serde::de;
use serde::de::Visitor;
use serde::Deserializer;
use std::fmt;

pub fn is_false(b: &bool) -> bool {
    return !b;
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_i64(BooleanVisitor)
}

struct BooleanVisitor;

impl<'de> Visitor<'de> for BooleanVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a number")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v == 1)
    }
}
