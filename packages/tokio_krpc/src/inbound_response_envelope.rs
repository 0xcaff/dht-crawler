use krpc_encoding as proto;

/// Inbound response sent from another node associated with an earlier query
/// originating from this node
pub struct InboundResponseEnvelope {
    pub transaction_id: Vec<u8>,
    pub response: ResponseType,
}

pub enum ResponseType {
    Error { error: proto::KRPCError },
    Response { response: proto::Response },
}
