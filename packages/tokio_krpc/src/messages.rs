use krpc_encoding::{
    self as proto,
    Query,
};

pub enum PortType {
    Implied,
    Port(u16),
}

#[derive(Debug)]
pub struct Request {
    pub transaction_id: Vec<u8>,
    pub version: Option<Vec<u8>>,
    pub query: Query,
    pub read_only: bool,
}

impl Request {
    pub fn new(transaction_id: Vec<u8>, query: proto::Query, read_only: bool) -> Request {
        Request {
            transaction_id,
            version: None,
            query,
            read_only,
        }
    }
}
