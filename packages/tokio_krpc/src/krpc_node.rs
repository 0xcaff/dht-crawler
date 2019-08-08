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
    TryStream,
    TryStreamExt,
};
use krpc_encoding::Message;
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

/// Handles making queries to other nodes, receiving responses and processing
/// queries from other nodes
pub struct KRPCNode {
    send_half: UdpSocketSendHalf,
    recv_half: UdpSocketRecvHalf,
    transactions: ActiveTransactions,
}

impl KRPCNode {
    pub fn new(socket: UdpSocket) -> KRPCNode {
        let (recv_half, send_half) = socket.split();
        let transactions = ActiveTransactions::new();

        KRPCNode {
            send_half,
            recv_half,
            transactions,
        }
    }

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
        impl TryStream<Ok = (InboundQuery, SocketAddr), Error = Error>,
    ) {
        let transactions = self.transactions.clone();

        let query_stream = receive_inbound_messages(self.recv_half)
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
            SendTransport::new(self.send_half, self.transactions),
            query_stream,
        )
    }
}
