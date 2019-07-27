use crate::{
    active_transactions::ActiveTransactions,
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    inbound_message_stream::receive_inbound_messages,
    messages::Request,
    SendTransport,
};
use failure::ResultExt;
use futures::{
    future,
    stream,
    TryStream,
    TryStreamExt,
};
use krpc_encoding::{
    Envelope,
    Message,
};
use std::{
    self,
    net::SocketAddr,
};
use tokio::{
    self,
    net::udp::{
        split::{
            UdpSocketRecvHalf,
            UdpSocketSendHalf,
        },
        UdpSocket,
    },
};

pub struct RecvTransport {
    send_half: UdpSocketSendHalf,
    recv_half: UdpSocketRecvHalf,
    transactions: ActiveTransactions,
}

impl RecvTransport {
    pub fn new(bind_address: SocketAddr) -> Result<RecvTransport> {
        let socket =
            UdpSocket::bind(&bind_address).map_err(|cause| ErrorKind::BindError { cause })?;

        let (recv_half, send_half) = socket.split();
        let transactions = ActiveTransactions::new();

        Ok(RecvTransport {
            send_half,
            recv_half,
            transactions,
        })
    }

    pub fn serve(
        self,
    ) -> (
        SendTransport,
        impl TryStream<Ok = (Request, SocketAddr), Error = Error>,
    ) {
        let transactions = self.transactions.clone();

        let query_stream = Self::receive_inbound_messages(self.recv_half)
            .map_ok(move |(envelope, from_addr)| match envelope.message_type {
                Message::Response { .. } | Message::Error { .. } => {
                    transactions.handle_response(envelope)?;

                    Ok(None)
                }
                Message::Query { query } => Ok(Some((
                    Request::new(envelope.transaction_id, query, envelope.read_only),
                    from_addr,
                ))),
            })
            .try_filter_map(|result| future::ready(result));

        (
            SendTransport::new(self.send_half, self.transactions),
            query_stream,
        )
    }

    fn receive_inbound_messages(
        recv_socket: UdpSocketRecvHalf,
    ) -> impl TryStream<Ok = (Envelope, SocketAddr), Error = Error> {
        let recv_buffer = [0 as u8; 1024];

        stream::unfold((recv_socket, recv_buffer), |(recv_socket, recv_buffer)| {
            receive_inbound_message_wrapper(recv_socket, recv_buffer)
        })
    }
}

async fn receive_inbound_message_wrapper(
    mut recv_socket: UdpSocketRecvHalf,
    mut recv_buffer: [u8; 1024],
) -> Option<(
    Result<(Envelope, SocketAddr)>,
    (UdpSocketRecvHalf, [u8; 1024]),
)> {
    let result = receive_inbound_message(&mut recv_socket, &mut recv_buffer).await;

    Some((result, (recv_socket, recv_buffer)))
}

async fn receive_inbound_message(
    recv_socket: &mut UdpSocketRecvHalf,
    recv_buffer: &mut [u8; 1024],
) -> Result<(Envelope, SocketAddr)> {
    let (size, from_addr) = recv_socket
        .recv_from(recv_buffer)
        .await
        .map_err(|cause| ErrorKind::FailedToReceiveMessage { cause })?;

    let envelope = Envelope::decode(&recv_buffer[..size]).with_context(|_| {
        ErrorKind::InvalidInboundMessage {
            from: from_addr,
            message: recv_buffer[..size].to_vec(),
        }
    })?;

    Ok((envelope, from_addr))
}
