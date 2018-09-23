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

use byteorder::NetworkEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

use bytes::BytesMut;

#[derive(Debug)]
pub struct Request {
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

#[derive(Debug)]
pub struct Response {
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
                })?;
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

    fn parse(src: &[u8]) -> Result<Response> {
        let envelope: Envelope = Envelope::decode(&src).context(ErrorKind::InvalidResponse)?;
        Ok(Response::from(envelope)?)
    }
}

#[cfg(test)]
mod tests {
    use client::Request;
    use client::Response;
    use proto::Query;
    use std::net::Ipv4Addr;
    use std::net::SocketAddrV4;
    use std::net::UdpSocket;

    #[test]
    fn test_ping() {
        let mut socket = UdpSocket::bind("0.0.0.0:34254").unwrap();
        let bootstrap_node = "router.bittorrent.com:6881";
        socket.connect(bootstrap_node);

        let transaction_id = 0x8a;
        let req = Request {
            transaction_id,
            version: None,
            query: Query::Ping {
                id: b"abcdefghij0123456789".into(),
            },
        };

        let req_encoded = req.into().unwrap().encode().unwrap();
        socket.send(&req_encoded).unwrap();

        let mut recv_buffer = [0 as u8; 1024];
        socket.recv(&mut recv_buffer).unwrap();

        let resp = Response::parse(&recv_buffer).unwrap();

        assert_eq!(resp.transaction_id, transaction_id);
    }
}
