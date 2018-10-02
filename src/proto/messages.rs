use serde_bencode;
use serde_bytes;

use std::fmt;

use super::node_info;
use super::{Addr, NodeID, NodeInfo};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// Public IP address of the requester. Only sent by peers supporting BEP42.
    pub ip: Option<Addr>,

    #[serde(rename = "t", with = "serde_bytes")]
    pub transaction_id: Vec<u8>,

    #[serde(rename = "v")]
    pub version: Option<String>,

    #[serde(flatten)]
    pub message_type: MessageType,
}

impl Envelope {
    pub fn decode(bytes: &[u8]) -> serde_bencode::Result<Envelope> {
        serde_bencode::de::from_bytes(bytes)
    }

    pub fn encode(&self) -> serde_bencode::Result<Vec<u8>> {
        serde_bencode::ser::to_bytes(self)
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
        error: Error,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Error(u8, String);

impl Error {
    pub fn new(error_code: u8, message: String) -> Error {
        Error(error_code, message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
        implied_port: u8,
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

        /// Empty when the responder decides we are unfit to send AnnouncePeer messages by BEP42.
        token: Option<Vec<u8>>,

        #[serde(with = "node_info")]
        nodes: Vec<NodeInfo>,
    },
    GetPeers {
        id: NodeID,

        /// Empty when the responder decides we are unfit to send AnnouncePeer messages by BEP42.
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
