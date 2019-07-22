use crate::{
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    proto::MessageType,
    transport::{
        active_transactions::ActiveTransactions,
        inbound_message_stream::InboundMessageStream,
        messages::Request,
        SendTransport,
    },
};
use failure::ResultExt;
use std::{
    self,
    net::SocketAddr,
    sync::Arc,
};
use tokio::{
    self,
    prelude::*,
};
use tokio_udp::UdpSocket;

pub struct RecvTransport {
    socket: Arc<UdpSocket>,
    transactions: ActiveTransactions,
}

impl RecvTransport {
    pub fn new(bind_address: SocketAddr) -> Result<RecvTransport> {
        let socket = Arc::new(UdpSocket::bind(&bind_address).context(ErrorKind::BindError)?);
        let transactions = ActiveTransactions::new();

        Ok(RecvTransport {
            socket,
            transactions,
        })
    }

    pub fn serve_read_only(
        self,
    ) -> (
        SendTransport,
        impl Stream<Item = (Request, SocketAddr), Error = Error>,
    ) {
        self.serve_impl(true)
    }

    pub fn serve(
        self,
    ) -> (
        SendTransport,
        impl Stream<Item = (Request, SocketAddr), Error = Error>,
    ) {
        self.serve_impl(false)
    }

    fn serve_impl(
        self,
        read_only: bool,
    ) -> (
        SendTransport,
        impl Stream<Item = (Request, SocketAddr), Error = Error>,
    ) {
        let transactions = self.transactions.clone();

        let query_stream = InboundMessageStream::new(self.socket.clone())
            .map(move |(envelope, from_addr)| match envelope.message_type {
                MessageType::Response { .. } | MessageType::Error { .. } => {
                    transactions.handle_response(envelope)?;

                    Ok(None)
                }
                MessageType::Query { query } => Ok(Some((
                    Request::new(envelope.transaction_id, query, envelope.read_only),
                    from_addr,
                ))),
            })
            .and_then(|r| r.into_future())
            .filter_map(|m| m);

        (
            SendTransport::new(self.socket, self.transactions, read_only),
            query_stream,
        )
    }
}
