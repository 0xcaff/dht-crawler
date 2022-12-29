use hex;
use num_bigint::BigUint;
use num_traits::One;
use rand;
use serde::{
    de::{
        self,
        Visitor,
    },
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};
use std::{
    fmt,
    ops::Deref,
};

/// Value representing a key or node ID in the DHT
#[derive(PartialEq, Eq, Clone, Hash)]
pub struct NodeID(BigUint);

pub const NODE_ID_SIZE_BITS: usize = 20 * 8;

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

    /// Returns true if the value of the nth bit is 1. The 0th bit is the most
    /// significant bit.
    pub fn nth_bit(&self, n: usize) -> bool {
        let one = BigUint::one();
        return ((self.deref() >> n) & &one) == one;
    }
}

impl Deref for NodeID {
    type Target = BigUint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for NodeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl fmt::Display for NodeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    use num_bigint::BigUint;

    #[test]
    fn as_bytes() {
        let id = NodeID::new(BigUint::from(1u8));
        let bytes = id.as_bytes();
        let mut expected = [0u8; 20];
        expected[0] = 1;

        assert_eq!(bytes, expected);
    }

    #[test]
    fn first_bit() {
        ensure_bits_for(
            b"8b9292b2f75d127720ebcd8afe66bfa50c2adc7f".into(),
            "10001011 10010010 10010010 10110010 11110111 01011101 00010010 01110111 00100000 11101011 11001101 10001010 11111110 01100110 10111111 10100101 00001100 00101010 11011100 01111111"
        )
    }

    fn ensure_bits_for(id: NodeID, expected_bits: &str) {
        let mut bit_strings = (0..160)
            .map(|n| id.nth_bit(n))
            .map(|n| if n { "1" } else { "0" })
            .map(|s| String::from(s))
            .collect::<Vec<String>>();

        bit_strings.reverse();

        let actual_bits = bit_strings.join("");

        assert_eq!(actual_bits, expected_bits.replace(" ", ""))
    }
}
