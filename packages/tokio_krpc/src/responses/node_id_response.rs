use crate::send_errors::{
    ErrorKind,
    Result,
};

use krpc_encoding::{
    self as proto,
    NodeID,
};

pub struct NodeIDResponse;

impl NodeIDResponse {
    pub fn from_response(response: proto::Response) -> Result<NodeID> {
        Ok(match response {
            proto::Response::OnlyID { id } => id,
            got => Err(ErrorKind::InvalidResponseType {
                expected: "NodeIDResponse",
                got,
            })?,
        })
    }
}
