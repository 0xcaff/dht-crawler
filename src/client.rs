use errors::Error;
use errors::ErrorKind;
use errors::Result;
use failure::ResultExt;

use proto;
use proto::Envelope;
use proto::MessageType;
use proto::Query;

use tokio::codec::Decoder;
use tokio::codec::Encoder;
use tokio::prelude::*;

use byteorder::NetworkEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

use bytes::BytesMut;

struct Request {
    transaction_id: u16,
    version: Option<String>,
    query: Query,
}

impl Request {
    fn into(self) -> Result<Envelope> {
        let mut transaction_id = Vec::new();
        transaction_id
            .write_u16::<NetworkEndian>(self.transaction_id)
            .expect("converting request txid for envelope");

        Ok(Envelope {
            transaction_id,
            version: self.version,
            message_type: MessageType::Query { query: self.query },
        })
    }
}

struct Response {
    transaction_id: u16,
    version: Option<String>,
    response: proto::Response,
}

impl Response {
    fn from(envelope: Envelope) -> Result<Response> {
        let response = match envelope.message_type {
            MessageType::Error { error } => {
                return Err(ErrorKind::PeerError {
                    protocol_error: error,
                })?
            }
            MessageType::Query { .. } => return Err(ErrorKind::InvalidResponse)?,
            MessageType::Response { response } => response,
        };

        Ok(Response {
            transaction_id: (&envelope.transaction_id[..])
                .read_u16::<NetworkEndian>()
                .context(ErrorKind::InvalidResponse)?,
            version: envelope.version,
            response,
        })
    }
}

struct ClientCodec;

impl Decoder for ClientCodec {
    type Item = Response;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        let envelope: Envelope = Envelope::decode(&src).context(ErrorKind::InvalidResponse)?;
        Ok(Some(Response::from(envelope)?))
    }
}

impl Encoder for ClientCodec {
    type Item = Request;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<()> {
        let encoded = item
            .into()?
            .encode()
            .context(ErrorKind::EncodingRequestFailed)?;

        (&mut dst[..])
            .write_all(&encoded)
            .context(ErrorKind::EncodingRequestFailed)?;

        Ok(())
    }
}
