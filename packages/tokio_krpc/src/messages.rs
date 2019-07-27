use crate::errors::{
    ErrorKind,
    Result,
};
use byteorder::{
    NetworkEndian,
    ReadBytesExt,
};
use failure::ResultExt;
use krpc_encoding::{
    self as proto,
    Envelope,
    Message,
    Query,
};

/// Transaction identifier used for requests originating from this client.
/// Requests originating from other clients use a `Vec<u8>` to represent the
/// transaction id.
pub type TransactionId = u32;

/// Extracts a [TransactionId] from a response to a request originating from
/// this client. If the transaction id is malformed, returns an error.
pub fn parse_originating_transaction_id(mut bytes: &[u8]) -> Result<TransactionId> {
    if bytes.len() != 4 {
        Err(ErrorKind::InvalidResponseTransactionId)?;
    }

    Ok(bytes
        .read_u32::<NetworkEndian>()
        .context(ErrorKind::InvalidResponseTransactionId)?)
}

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

    pub fn into(self) -> Envelope {
        Envelope {
            ip: None,
            transaction_id: self.transaction_id,
            version: self.version.map(|version| version.into()),
            message_type: Message::Query { query: self.query },
            read_only: self.read_only,
        }
    }
}
