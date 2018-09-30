use rand;

use serde::de;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

use std::fmt;

use hex;

/// A 20 byte value representing keys in the DHT.
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct NodeID([u8; 20]);

impl NodeID {
    pub fn random() -> NodeID {
        rand::random::<[u8; 20]>().into()
    }

    pub fn from_bytes(bytes: &[u8]) -> NodeID {
        let mut cloned = [0u8; 20];
        cloned.copy_from_slice(&bytes[..20]);

        NodeID(cloned)
    }

    pub fn from_hex(bytes: &[u8; 40]) -> NodeID {
        let raw: &[u8] = bytes;
        let bytes = hex::decode(raw).unwrap();

        NodeID::from_bytes(&bytes)
    }

    pub fn get_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl fmt::Debug for NodeID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.get_bytes()))
    }
}

impl Serialize for NodeID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
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
        NodeID(bytes.clone())
    }
}

impl<'a> From<&'a [u8; 40]> for NodeID {
    fn from(bytes: &[u8; 40]) -> Self {
        NodeID::from_hex(bytes)
    }
}

impl From<[u8; 20]> for NodeID {
    fn from(arr: [u8; 20]) -> Self {
        NodeID(arr)
    }
}
