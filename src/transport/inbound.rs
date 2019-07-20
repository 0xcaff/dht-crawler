use crate::{
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    proto::Message,
};
use failure::ResultExt;
use futures::try_ready;
use std::net::SocketAddr;
use tokio::{
    self,
    prelude::*,
};

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
    type Item = (Message, SocketAddr);
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>> {
        let mut recv_buffer = [0 as u8; 1024];

        let (size, from_addr) = try_ready!(self
            .recv_socket
            .poll_recv_from(&mut recv_buffer)
            .context(ErrorKind::BindError));

        let envelope = Message::decode(&recv_buffer[..size]).with_context(|_| {
            ErrorKind::InvalidInboundMessage {
                from: from_addr,
                message: recv_buffer[..size].to_vec(),
            }
        })?;

        Ok(Async::Ready(Some((envelope, from_addr))))
    }
}
