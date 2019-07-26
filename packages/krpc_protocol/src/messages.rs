use super::{
    booleans,
    node_info,
    Addr,
    NodeID,
    NodeInfo,
};
use crate::errors::{
    ErrorKind,
    Result,
};
use serde_bencode;
use serde_bytes::{
    self,
    ByteBuf,
};
use serde_derive::{
    Deserialize,
    Serialize,
};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Message {
    /// Public IP address of the requester. Only sent by peers supporting BEP42.
    pub ip: Option<Addr>,

    #[serde(rename = "t", with = "serde_bytes")]
    pub transaction_id: Vec<u8>,

    #[serde(rename = "v")]
    pub version: Option<ByteBuf>,

    #[serde(flatten)]
    pub message_type: MessageType,

    /// From BEP43.
    #[serde(
        rename = "ro",
        default,
        skip_serializing_if = "booleans::is_false",
        deserialize_with = "booleans::deserialize"
    )]
    pub read_only: bool,
}

impl Message {
    pub fn decode(bytes: &[u8]) -> Result<Message> {
        Ok(serde_bencode::de::from_bytes(bytes)
            .map_err(|cause| ErrorKind::DecodeError { cause })?)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        Ok(serde_bencode::ser::to_bytes(self).map_err(|cause| ErrorKind::EncodeError { cause })?)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "y")]
pub enum MessageType {
    #[serde(rename = "q")]
    Query {
        #[serde(flatten)]
        query: Query,
    },

    #[serde(rename = "r")]
    Response {
        #[serde(rename = "r")]
        response: Response,
    },

    #[serde(rename = "e")]
    Error {
        #[serde(rename = "e")]
        error: ProtocolError,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ProtocolError(u8, String);

impl ProtocolError {
    pub fn new(error_code: u8, message: &str) -> ProtocolError {
        ProtocolError(error_code, message.to_string())
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "q", content = "a")]
pub enum Query {
    #[serde(rename = "ping")]
    Ping { id: NodeID },

    #[serde(rename = "find_node")]
    FindNode { id: NodeID, target: NodeID },

    #[serde(rename = "get_peers")]
    GetPeers { id: NodeID, info_hash: NodeID },

    #[serde(rename = "announce_peer")]
    AnnouncePeer {
        id: NodeID,

        #[serde(deserialize_with = "booleans::deserialize")]
        implied_port: bool,
        port: Option<u16>,
        info_hash: NodeID,

        #[serde(with = "serde_bytes")]
        token: Vec<u8>,
    },

    /// `sample_infohashes` request from BEP51.
    #[serde(rename = "sample_infohashes")]
    SampleInfoHashes { id: NodeID, target: NodeID },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Response {
    NextHop {
        id: NodeID,

        /// Empty when the responder decides we are unfit to send AnnouncePeer
        /// messages by BEP42.
        token: Option<Vec<u8>>,

        #[serde(with = "node_info")]
        nodes: Vec<NodeInfo>,
    },
    GetPeers {
        id: NodeID,

        /// Empty when the responder decides we are unfit to send AnnouncePeer
        /// messages by BEP42.
        token: Option<Vec<u8>>,

        #[serde(rename = "values")]
        peers: Vec<Addr>,
    },
    /// Sent in response to Ping and AnnouncePeer
    OnlyId { id: NodeID },

    /// Response to SampleInfoHashes from BEP51.
    Samples {
        /// Identifier of sending node
        id: NodeID,

        /// Number of seconds this node should not be queried again for
        interval: Option<u16>,

        /// Nodes close to target in request
        #[serde(with = "node_info")]
        nodes: Vec<NodeInfo>,

        /// Number of info hashes this peer has
        num: Option<u32>,

        /// Sample of info-hashes
        samples: Vec<NodeID>,
    },
}
