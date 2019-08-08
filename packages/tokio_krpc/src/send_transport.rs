use crate::{
    active_transactions::ActiveTransactions,
    response_future::ResponseFuture,
    send_errors::{
        ErrorKind,
        Result,
    },
    transaction_id::TransactionId,
};
use futures::lock::Mutex;
use krpc_encoding::{
    self as proto,
    Envelope,
    Message,
    Query,
};
use std::net::SocketAddr;
use tokio::net::udp::split::UdpSocketSendHalf;

/// Low-level wrapper around a UDP socket for sending KRPC queries and
/// responses.
pub struct SendTransport {
    socket: Mutex<UdpSocketSendHalf>,
    transactions: ActiveTransactions,
}

impl SendTransport {
    pub(crate) fn new(
        socket: UdpSocketSendHalf,
        transactions: ActiveTransactions,
    ) -> SendTransport {
        SendTransport {
            socket: Mutex::new(socket),
            transactions,
        }
    }

    /// Encodes and sends `message` to `address` without waiting for a response.
    pub async fn send(&self, address: SocketAddr, message: Envelope) -> Result<()> {
        let encoded = message
            .encode()
            .map_err(|cause| ErrorKind::SendEncodingError { cause })?;

        let mut socket = self.socket.lock().await;

        socket
            .send_to(&encoded, &address)
            .await
            .map_err(|cause| ErrorKind::SendError { cause })?;

        Ok(())
    }

    pub async fn request(&self, address: SocketAddr, query: Query) -> Result<proto::Response> {
        let transaction_id = Self::random_transaction_id();

        let envelope = Envelope {
            ip: None,
            transaction_id: transaction_id.to_be_bytes().to_vec(),
            version: None,
            message_type: Message::Query { query },
            read_only: false,
        };

        self.send(address, envelope).await?;

        Ok(ResponseFuture::wait_for_tx(transaction_id, self.transactions.clone()).await?)
    }

    fn random_transaction_id() -> TransactionId {
        rand::random::<TransactionId>()
    }
}
