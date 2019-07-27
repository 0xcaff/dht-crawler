use crate::{
    active_transactions::ActiveTransactions,
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    inbound::receive_inbound_messages,
    inbound_response_envelope::{
        InboundResponseEnvelope,
        ResponseType,
    },
    messages::Request,
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

pub struct KRPCNode {
    send_half: UdpSocketSendHalf,
    recv_half: UdpSocketRecvHalf,
    transactions: ActiveTransactions,
}

impl KRPCNode {
    pub fn bind(bind_address: SocketAddr) -> Result<KRPCNode> {
        let socket =
            UdpSocket::bind(&bind_address).map_err(|cause| ErrorKind::BindError { cause })?;

        let (recv_half, send_half) = socket.split();
        let transactions = ActiveTransactions::new();

        Ok(KRPCNode {
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
}
