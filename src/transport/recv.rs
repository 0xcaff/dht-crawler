use errors::{Error, ErrorKind, Result};
use failure::ResultExt;

use std;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio;
use tokio::prelude::*;
use tokio::reactor::Handle;

use proto::MessageType;
use transport::inbound::InboundMessageStream;
use transport::messages::Request;
use transport::response::{ResponseFuture, TransactionMap};
use transport::SendTransport;

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

    pub fn serve(
        self,
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
                    Request::new(envelope.transaction_id, query),
                    from_addr,
                ))),
            }).and_then(|r| r.into_future())
            .filter_map(|m| m);

        (self.into_send_transport(), query_stream)
    }

    /// Consumes the `RecvTransport` and make a `SendTransport`
    fn into_send_transport(self) -> SendTransport {
        SendTransport::new(self.socket, self.transactions)
    }
}
