use errors::{Error, ErrorKind, Result};
use failure::ResultExt;

use std::{
    self,
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{self, prelude::*, reactor::Handle};

use proto::MessageType;
use transport::{
    inbound::InboundMessageStream,
    messages::Request,
    response::{ResponseFuture, TransactionMap},
    SendTransport,
};

pub struct RecvTransport {
    socket: std::net::UdpSocket,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl RecvTransport {
    pub fn new(bind_address: SocketAddr) -> Result<RecvTransport> {
        let socket = std::net::UdpSocket::bind(&bind_address).context(ErrorKind::BindError)?;
        let transactions = Arc::new(Mutex::new(HashMap::new()));

        Ok(RecvTransport {
            socket,
            transactions,
        })
    }

    fn make_tokio_socket(&self) -> Result<tokio::net::UdpSocket> {
        let cloned_socket = self.socket.try_clone().context(ErrorKind::BindError)?;
        let tokio_socket = tokio::net::UdpSocket::from_std(cloned_socket, &Handle::default())
            .context(ErrorKind::BindError)?;

        Ok(tokio_socket)
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

        let query_stream = self
            .make_tokio_socket()
            .into_future()
            .into_stream()
            .map(InboundMessageStream::new)
            .flatten()
            .map(move |(envelope, from_addr)| match envelope.message_type {
                MessageType::Response { .. } | MessageType::Error { .. } => {
                    ResponseFuture::handle_response(envelope, transactions.clone())?;

                    Ok(None)
                }
                MessageType::Query { query } => Ok(Some((
                    Request::new(envelope.transaction_id, query, envelope.read_only),
                    from_addr,
                ))),
            }).and_then(|r| r.into_future())
            .filter_map(|m| m);

        (
            SendTransport::new(self.socket, self.transactions, read_only),
            query_stream,
        )
    }
}
