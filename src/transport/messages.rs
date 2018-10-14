use errors::{ErrorKind, Result};
use failure::ResultExt;

use proto;
use proto::{Addr, Message, MessageType, NodeID, NodeInfo, Query};

use byteorder::{NetworkEndian, ReadBytesExt};
use std::net::SocketAddrV4;

pub type TransactionId = u32;

pub enum PortType {
    Implied,
    Port(u16),
}

#[derive(Debug)]
pub struct Request {
    pub transaction_id: Vec<u8>,
    pub version: Option<String>,
    pub query: Query,
}

impl Request {
    pub fn new(transaction_id: Vec<u8>, query: proto::Query) -> Request {
        Request {
            transaction_id,
            version: None,
            query,
        }
    }

    pub fn into(self) -> Message {
        Message {
            ip: None,
            transaction_id: self.transaction_id,
            version: self.version,
            message_type: MessageType::Query { query: self.query },
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub transaction_id: TransactionId,
    pub version: Option<String>,
    pub response: proto::Response,
}

impl Response {
    pub fn from(envelope: Message) -> Result<Response> {
        let response = match envelope.message_type {
            MessageType::Error { error } => {
                return Err(ErrorKind::PeerError {
                    protocol_error: error,
                })?;
            }
            MessageType::Query { .. } => return Err(ErrorKind::InvalidResponse)?,
            MessageType::Response { response } => response,
        };

        Ok(Response {
            transaction_id: (&envelope.transaction_id[..])
                .read_u32::<NetworkEndian>()
                .context(ErrorKind::InvalidResponse)?,
            version: envelope.version,
            response,
        })
    }

    pub fn parse(src: &[u8]) -> Result<Response> {
        let envelope: Message = Message::decode(&src).context(ErrorKind::InvalidResponse)?;
        Ok(Response::from(envelope)?)
    }
}

pub struct FindNodeResponse {
    pub id: NodeID,
    pub nodes: Vec<NodeInfo>,
}

impl FindNodeResponse {
    pub fn from_response(resp: Response) -> Result<FindNodeResponse> {
        Ok(match resp.response {
            proto::Response::NextHop { id, nodes, .. } => FindNodeResponse { id, nodes },
            _ => Err(ErrorKind::InvalidResponseType)?,
        })
    }
}

pub struct GetPeersResponse {
    pub id: NodeID,
    pub token: Option<Vec<u8>>,
    pub message_type: GetPeersResponseType,
}

impl GetPeersResponse {
    pub fn from_response(response: Response) -> Result<GetPeersResponse> {
        Ok(match response.response {
            proto::Response::GetPeers { id, token, peers } => GetPeersResponse {
                id,
                token,
                message_type: GetPeersResponseType::Peers(
                    peers.into_iter().map(Addr::into).collect(),
                ),
            },
            proto::Response::NextHop { id, token, nodes } => GetPeersResponse {
                id,
                token,
                message_type: GetPeersResponseType::NextHop(nodes),
            },
            _ => Err(ErrorKind::InvalidResponseType)?,
        })
    }
}

pub enum GetPeersResponseType {
    Peers(Vec<SocketAddrV4>),
    NextHop(Vec<NodeInfo>),
}

pub struct NodeIDResponse;

impl NodeIDResponse {
    pub fn from_response(resp: Response) -> Result<NodeID> {
        Ok(match resp.response {
            proto::Response::OnlyId { id } => id,
            _ => Err(ErrorKind::InvalidResponseType)?,
        })
    }
}
