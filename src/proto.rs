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

    #[test]
    fn get_nodes_response() {
        let parsed = Envelope {
            transaction_id: b"aa".to_vec(),
            version: None,
            message_type: MessageType::Response {
                response: Response::FindNode {
                    id: b"abcdefghij0123456789".into(),
                    nodes: Vec::new(),
                },
            },
        };

        let serialized = serde_bencode::ser::to_bytes(&parsed).unwrap();
        let decoded = Envelope::decode(&serialized).unwrap();

        assert_eq!(parsed, decoded);
    }

    #[test]
    fn get_nodes_response_decode() {
        let encoded: &[u8] = &[
            100, 50, 58, 105, 112, 54, 58, 129, 21, 63, 170, 133, 190, 49, 58, 114, 100, 50, 58,
            105, 100, 50, 48, 58, 50, 245, 78, 105, 115, 81, 255, 74, 236, 41, 205, 186, 171, 242,
            251, 227, 70, 124, 194, 103, 53, 58, 110, 111, 100, 101, 115, 52, 49, 54, 58, 48, 33,
            11, 23, 67, 40, 27, 83, 194, 152, 189, 83, 184, 116, 44, 224, 100, 119, 227, 172, 180,
            211, 234, 53, 5, 136, 247, 55, 4, 69, 93, 133, 57, 156, 104, 27, 0, 231, 29, 145, 49,
            172, 172, 170, 50, 51, 36, 37, 147, 240, 49, 120, 205, 9, 249, 147, 103, 202, 47, 147,
            118, 247, 56, 14, 110, 23, 186, 53, 174, 165, 170, 186, 95, 24, 216, 93, 124, 7, 192,
            112, 119, 16, 106, 92, 58, 112, 137, 128, 138, 141, 79, 23, 69, 24, 183, 4, 85, 166,
            93, 172, 43, 127, 90, 117, 12, 129, 47, 223, 197, 10, 15, 183, 213, 97, 35, 240, 235,
            237, 50, 252, 249, 194, 225, 219, 70, 124, 69, 205, 196, 145, 102, 100, 250, 166, 128,
            104, 68, 91, 140, 182, 54, 54, 90, 21, 2, 241, 200, 141, 23, 37, 46, 153, 74, 174, 251,
            147, 165, 79, 20, 85, 75, 125, 77, 206, 96, 25, 32, 99, 225, 224, 103, 85, 243, 146,
            250, 181, 81, 97, 116, 190, 26, 225, 222, 157, 234, 191, 56, 113, 115, 126, 188, 27,
            149, 83, 240, 151, 53, 226, 74, 241, 83, 226, 84, 251, 160, 222, 188, 171, 86, 40, 168,
            238, 141, 18, 184, 130, 83, 38, 118, 45, 28, 54, 40, 41, 156, 202, 216, 46, 98, 13, 2,
            205, 26, 225, 63, 156, 12, 215, 19, 180, 67, 243, 186, 19, 109, 221, 5, 80, 152, 247,
            35, 243, 248, 56, 42, 98, 51, 123, 36, 88, 116, 101, 114, 42, 208, 241, 77, 164, 158,
            29, 72, 206, 241, 52, 116, 105, 188, 110, 109, 117, 79, 114, 47, 76, 250, 186, 139,
            146, 146, 178, 247, 93, 18, 119, 32, 235, 205, 138, 254, 102, 191, 165, 12, 42, 220,
            127, 2, 87, 195, 123, 244, 241, 208, 251, 133, 56, 218, 180, 25, 130, 48, 88, 121, 190,
            163, 198, 23, 107, 74, 12, 187, 222, 49, 70, 2, 154, 62, 129, 127, 66, 65, 164, 135,
            151, 240, 82, 208, 230, 231, 249, 209, 128, 98, 123, 231, 28, 218, 245, 70, 55, 32,
            213, 70, 20, 52, 38, 230, 211, 179, 139, 75, 33, 144, 222, 204, 108, 131, 204, 243,
            102, 133, 52, 64, 145, 124, 77, 137, 19, 62, 129, 9, 0, 237, 24, 24, 39, 3, 64, 227,
            246, 41, 203, 19, 170, 174, 98, 102, 66, 33, 245, 119, 237, 152, 161, 26, 234, 101, 49,
            58, 116, 52, 58, 0, 0, 175, 218, 49, 58, 121, 49, 58, 114, 101,
        ];

        let envelope = Envelope::decode(encoded).unwrap();

        println!("{:#?}", envelope);

        match envelope.message_type {
            MessageType::Response {
                response: Response::GetPeers { .. },
            } => (),
            _ => panic!("Invalid Message Type Found"),
        };
    }
}
