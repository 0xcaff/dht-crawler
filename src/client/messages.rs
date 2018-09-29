use errors::{ErrorKind, Result};
use failure::ResultExt;

use proto;
use proto::{Envelope, MessageType, Query};

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

pub type TransactionId = u32;

#[derive(Debug)]
pub struct Request {
    pub transaction_id: TransactionId,
    pub version: Option<String>,
    pub query: Query,
}

impl Request {
    pub fn into(self) -> Result<Envelope> {
        let mut transaction_id = Vec::new();
        transaction_id
            .write_u32::<NetworkEndian>(self.transaction_id)
            .expect("converting request txid for envelope");

        Ok(Envelope {
            transaction_id,
            version: self.version,
            message_type: MessageType::Query { query: self.query },
        })
    }

    pub fn encode(self) -> Result<Vec<u8>> {
        Ok(self.into()?.encode().context(ErrorKind::EncodeError)?)
    }
}

#[derive(Debug)]
pub struct Response {
    pub transaction_id: TransactionId,
    pub version: Option<String>,
    pub response: proto::Response,
}

impl Response {
    pub fn from(envelope: Envelope) -> Result<Response> {
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
        let envelope: Envelope = Envelope::decode(&src).context(ErrorKind::InvalidResponse)?;
        Ok(Response::from(envelope)?)
    }
}
