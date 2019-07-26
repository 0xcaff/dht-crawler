use crate::{
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    proto::MessageType,
    transport::{
        active_transactions::ActiveTransactions,
        inbound_message_stream::receive_inbound_messages,
        messages::Request,
        SendTransport,
    },
};
use failure::ResultExt;
use futures::{
    future,
    TryStream,
    TryStreamExt,
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
        let socket = UdpSocket::bind(&bind_address).context(ErrorKind::BindError)?;
        let (recv_half, send_half) = socket.split();
        let transactions = ActiveTransactions::new();

        Ok(RecvTransport {
            send_half,
            recv_half,
            transactions,
        })
    }

    pub fn serve_read_only(
        self,
    ) -> (
        SendTransport,
        impl TryStream<Ok = (Request, SocketAddr), Error = Error>,
    ) {
        self.serve_impl(true)
    }

    pub fn serve(
        self,
    ) -> (
        SendTransport,
        impl TryStream<Ok = (Request, SocketAddr), Error = Error>,
    ) {
        self.serve_impl(false)
    }

    fn serve_impl(
        self,
        read_only: bool,
    ) -> (
        SendTransport,
        impl TryStream<Ok = (Request, SocketAddr), Error = Error>,
    ) {
        let transactions = self.transactions.clone();

        let query_stream = receive_inbound_messages(self.recv_half)
            .map_ok(move |(envelope, from_addr)| match envelope.message_type {
                MessageType::Response { .. } | MessageType::Error { .. } => {
                    transactions.handle_response(envelope)?;

                    Ok(None)
                }
                MessageType::Query { query } => Ok(Some((
                    Request::new(envelope.transaction_id, query, envelope.read_only),
                    from_addr,
                ))),
            })
            .try_filter_map(|result| future::ready(result));

        (
            SendTransport::new(self.send_half, self.transactions, read_only),
            query_stream,
        )
    }
}
