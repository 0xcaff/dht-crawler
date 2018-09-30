use serde_bytes;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NodeID(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<Vec<u8>> for NodeID {
    fn from(bytes: Vec<u8>) -> Self {
        NodeID(bytes)
    }
}

impl<'a> From<&'a [u8; 20]> for NodeID {
    fn from(bytes: &[u8; 20]) -> Self {
        NodeID(bytes.to_vec())
    }
}
