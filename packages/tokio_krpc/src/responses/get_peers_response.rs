use crate::send_errors::{
    ErrorKind,
    Result,
};

use krpc_encoding::{
    self as proto,
    Addr,
    NodeID,
    NodeInfo,
};
use std::net::SocketAddrV4;

pub struct GetPeersResponse {
    pub id: NodeID,
    pub token: Option<Vec<u8>>,
    pub message_type: GetPeersResponseType,
}

impl GetPeersResponse {
    pub fn from_response(response: proto::Response) -> Result<GetPeersResponse> {
        Ok(match response {
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
            got => Err(ErrorKind::InvalidResponseType {
                expected: "GetPeersResponse (GetPeers or NextHop)",
                got,
            })?,
        })
    }
}

pub enum GetPeersResponseType {
    Peers(Vec<SocketAddrV4>),
    NextHop(Vec<NodeInfo>),
}
