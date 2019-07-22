use crate::{
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    proto::Message,
};
use failure::ResultExt;
use futuresx::{
    ready,
    TryStream,
};
use std::{
    net::SocketAddr,
    pin::Pin,
};
use tokio::{
    self,
    net::udp::split::UdpSocketRecvHalf,
    prelude::{
        task::Context,
        *,
    },
};

/// A future which handles receiving messages for the local peer.
pub struct InboundMessageStream {
    /// Socket for receiving messages from other peers
    recv_socket: UdpSocketRecvHalf,
}

impl InboundMessageStream {
    pub fn new(recv_socket: UdpSocketRecvHalf) -> InboundMessageStream {
        InboundMessageStream { recv_socket }
    }
}

impl TryStream for InboundMessageStream {
    type Ok = (Message, SocketAddr);
    type Error = Error;

    fn try_poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Self::Ok>>> {
        let mut recv_buffer = [0 as u8; 1024];

        let (size, from_addr) = ready!(self.recv_socket.poll_recv_from(cx, &mut recv_buffer))
            .context(ErrorKind::BindError)?;

        let envelope = Message::decode(&recv_buffer[..size]).with_context(|_| {
            ErrorKind::InvalidInboundMessage {
                from: from_addr,
                message: recv_buffer[..size].to_vec(),
            }
        })?;

        Poll::Ready(Some(Ok((envelope, from_addr))))
    }
}
