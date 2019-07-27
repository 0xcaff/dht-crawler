use krpc_encoding::{
    self as proto,
    Query,
};

/// Inbound query originating from another node
#[derive(Debug)]
pub struct InboundQuery {
    pub transaction_id: Vec<u8>,
    pub version: Option<Vec<u8>>,
    pub query: Query,
    pub read_only: bool,
}

impl InboundQuery {
    pub fn new(transaction_id: Vec<u8>, query: proto::Query, read_only: bool) -> InboundQuery {
        InboundQuery {
            transaction_id,
            version: None,
            query,
            read_only,
        }
    }
}
