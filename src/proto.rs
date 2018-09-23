use node_info::NodeInfo;
use serde_bencode;
use serde_bytes;
use std::fmt;

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
    fn new(error_code: u8, message: String) -> Error {
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

#[cfg(test)]
mod tests {
    use proto::{Envelope, Error, MessageType, Query, Response};
    use serde_bencode;
    use std::str;

    fn test_serialize_deserialize(parsed: Envelope, raw: &[u8]) {
        let serialized = serde_bencode::ser::to_string(&parsed).unwrap();
        let raw_string = str::from_utf8(raw).unwrap().to_string();

        assert_eq!(raw_string, serialized);
        assert_eq!(parsed, serde_bencode::de::from_bytes(raw).unwrap());
    }

    #[test]
    fn ping_request() {
        let parsed = Envelope {
            transaction_id: b"aa".to_vec(),
            version: None,
            message_type: MessageType::Query {
                query: Query::Ping {
                    id: b"abcdefghij0123456789".into(),
                },
            },
        };

        let raw = b"d1:ad2:id20:abcdefghij0123456789e1:q4:ping1:t2:aa1:y1:qe";
        test_serialize_deserialize(parsed, raw);
    }

    #[test]
    fn ping_response() {
        let parsed = Envelope {
            transaction_id: b"aa".to_vec(),
            version: None,
            message_type: MessageType::Response {
                response: Response::OnlyId {
                    id: b"mnopqrstuvwxyz123456".into(),
                },
            },
        };

        let raw = b"d1:rd2:id20:mnopqrstuvwxyz123456e1:t2:aa1:y1:re";
        test_serialize_deserialize(parsed, raw);
    }

    #[test]
    fn error() {
        let parsed = Envelope {
            transaction_id: b"aa".to_vec(),
            version: None,
            message_type: MessageType::Error {
                error: Error::new(201, "A Generic Error Ocurred".to_string()),
            },
        };

        let raw = b"d1:eli201e23:A Generic Error Ocurrede1:t2:aa1:y1:ee";
        test_serialize_deserialize(parsed, raw);
    }

    #[test]
    fn announce_peer_request() {
        let parsed = Envelope {
            transaction_id: b"aa".to_vec(),
            version: None,
            message_type: MessageType::Query {
                query: Query::AnnouncePeer {
                    id: b"abcdefghij0123456789".into(),
                    implied_port: 1,
                    port: Some(6881),
                    info_hash: b"mnopqrstuvwxyz123456".into(),
                    token: "aoeusnth".to_string(),
                },
            },
        };

        let raw = b"d1:ad2:id20:abcdefghij012345678912:implied_porti1e9:info_hash20:mnopqrstuvwxyz1234564:porti6881e5:token8:aoeusnthe1:q13:announce_peer1:t2:aa1:y1:qe";
        test_serialize_deserialize(parsed, raw);
    }
}
