use proto::Envelope;

use errors::{Error, ErrorKind, Result};
use failure::ResultExt;

use std::net::SocketAddr;
use tokio;
use tokio::prelude::*;

/// A future which handles receiving messages for the local peer.
pub struct InboundMessageStream {
    /// Socket for receiving messages from other peers
    recv_socket: tokio::net::UdpSocket,
}

impl InboundMessageStream {
    pub fn new(recv_socket: tokio::net::UdpSocket) -> InboundMessageStream {
        InboundMessageStream { recv_socket }
    }
}

impl Stream for InboundMessageStream {
    type Item = (Envelope, SocketAddr);
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>> {
        let mut recv_buffer = [0 as u8; 1024];

        let (size, from_addr) = try_ready!(
            self.recv_socket
                .poll_recv_from(&mut recv_buffer)
                .context(ErrorKind::BindError)
        );

        let envelope =
            Envelope::decode(&recv_buffer[..size]).context(ErrorKind::InvalidResponse)?;

        Ok(Async::Ready(Some((envelope, from_addr))))
    }
}
