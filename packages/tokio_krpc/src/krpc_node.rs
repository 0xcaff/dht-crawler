use crate::{
    active_transactions::ActiveTransactions,
    inbound::receive_inbound_messages,
    inbound_response_envelope::{
        InboundResponseEnvelope,
        ResponseType,
    },
    recv_errors::Error,
    InboundQuery,
    SendTransport,
};
use futures::{
    future,
    Stream,
    TryStreamExt,
};
use krpc_encoding::Message;
use std::{
    self,
    net::SocketAddr,
    sync::Arc,
};
use tokio::{
    self,
    net::UdpSocket,
};

/// Handles making queries to other nodes, receiving responses and processing
/// queries from other nodes
pub struct KRPCNode {
    socket: Arc<UdpSocket>,
    transactions: ActiveTransactions,
}

impl KRPCNode {
    pub fn new(socket: UdpSocket) -> KRPCNode {
        let transactions = ActiveTransactions::new();

        KRPCNode {
            socket: Arc::new(socket),
            transactions,
        }
    }

    // TODO: Separate the returned stream

    /// Starts listening for inbound queries and responses. The stream **MUST**
    /// be polled to process responses to outbound requests.
    ///
    /// # Returns
    /// A handle to send messages to other nodes and a stream of inbound
    /// requests. Errors occur on the stream whenever an error occurs while
    /// processing an inbound message.
    pub fn serve(
        self,
    ) -> (
        SendTransport,
        impl Stream<Item = Result<(InboundQuery, SocketAddr), Error>>,
    ) {
        let transactions = self.transactions.clone();

        let recv_half = self.socket.clone();
        let send_half = self.socket;

        let query_stream = receive_inbound_messages(recv_half)
            .map_ok(move |(envelope, from_addr)| match envelope.message_type {
                Message::Response { response } => {
                    transactions.handle_response(InboundResponseEnvelope {
                        transaction_id: envelope.transaction_id,
                        response: ResponseType::Response { response },
                    })?;

                    Ok(None)
                }
                Message::Error { error } => {
                    transactions.handle_response(InboundResponseEnvelope {
                        transaction_id: envelope.transaction_id,
                        response: ResponseType::Error { error },
                    })?;

                    Ok(None)
                }
                Message::Query { query } => Ok(Some((
                    InboundQuery::new(envelope.transaction_id, query, envelope.read_only),
                    from_addr,
                ))),
            })
            .try_filter_map(|result| future::ready(result));

        (
            SendTransport::new(send_half, self.transactions),
            query_stream,
        )
    }
}
