use serde_bencode;
use serde_bytes;

use std::fmt;

use super::{NodeID, NodeInfo};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Envelope {
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
        token: String,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum Port {
    Implied,
    Explicit(u16),
    None,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Response {
    /// Sent in response to Ping and AnnouncePeer
    OnlyId {
        id: NodeID,
    },
    FindNode {
        id: NodeID,
        nodes: Vec<NodeInfo>,
    },
    GetPeers {
        id: NodeID,
        token: Vec<u8>,
        peers: Vec<NodeInfo>,
    },
    NextHop {
        id: NodeID,
        token: Vec<u8>,
        nodes: Vec<NodeInfo>,
    },
}
