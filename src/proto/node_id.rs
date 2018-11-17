use rand;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

use std::{fmt, ops::Deref};

use bigint::BigUint;
use hex;

/// A 20 byte value representing keys in the DHT.
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct NodeID(BigUint);

impl NodeID {
    pub fn new(id: BigUint) -> NodeID {
        NodeID(id)
    }

    pub fn random() -> NodeID {
        rand::random::<[u8; 20]>().into()
    }

    pub fn from_bytes(bytes: &[u8]) -> NodeID {
        NodeID(BigUint::from_bytes_be(bytes))
    }

    pub fn from_hex(bytes: &[u8; 40]) -> NodeID {
        let raw: &[u8] = bytes;
        let bytes = hex::decode(raw).unwrap();

        NodeID::from_bytes(&bytes)
    }

    pub fn as_bytes(&self) -> [u8; 20] {
        let mut bytes = self.0.to_bytes_be();
        bytes.resize(20, 0);
        let mut output = [0u8; 20];
        output.copy_from_slice(&bytes[..]);

        output
    }
}

impl Deref for NodeID {
    type Target = BigUint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for NodeID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl fmt::Display for NodeID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_bytes()))
    }
}

impl Serialize for NodeID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.as_bytes())
    }
}

impl<'de> Deserialize<'de> for NodeID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(NodeIDVisitor)
    }
}

struct NodeIDVisitor;

impl<'de> Visitor<'de> for NodeIDVisitor {
    type Value = NodeID;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a byte array of size 20")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let len = v.len();
        if len != 20 {
            return Err(de::Error::invalid_length(len, &self));
        };

        Ok(NodeID::from_bytes(v))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(&v)
    }
}

impl<'a> From<&'a [u8; 20]> for NodeID {
    fn from(bytes: &[u8; 20]) -> Self {
        NodeID::from_bytes(bytes)
    }
}

impl<'a> From<&'a [u8; 40]> for NodeID {
    fn from(bytes: &[u8; 40]) -> Self {
        NodeID::from_hex(bytes)
    }
}

impl From<[u8; 20]> for NodeID {
    fn from(arr: [u8; 20]) -> Self {
        NodeID::from_bytes(&arr)
    }
}

#[cfg(test)]
mod tests {
    use super::NodeID;
    use bigint::BigUint;

    #[test]
    fn as_bytes() {
        let id = NodeID::new(BigUint::from(1u8));
        let bytes = id.as_bytes();
        let mut expected = [0u8; 20];
        expected[0] = 1;

        assert_eq!(bytes, expected);
    }
}
