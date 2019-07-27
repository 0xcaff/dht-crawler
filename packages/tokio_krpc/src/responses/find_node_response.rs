use crate::errors::{
    ErrorKind,
    Result,
};

use krpc_encoding::{
    self as proto,
    NodeID,
    NodeInfo,
};

pub struct FindNodeResponse {
    pub id: NodeID,
    pub nodes: Vec<NodeInfo>,
}

impl FindNodeResponse {
    pub fn from_response(response: proto::Response) -> Result<FindNodeResponse> {
        Ok(match response {
            proto::Response::NextHop { id, nodes, .. } => FindNodeResponse { id, nodes },
            got => Err(ErrorKind::InvalidResponseType {
                expected: "FindNodeResponse (NextHop)",
                got,
            })?,
        })
    }
}
