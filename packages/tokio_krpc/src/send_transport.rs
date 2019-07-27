use crate::{
    active_transactions::ActiveTransactions,
    errors::{
        ErrorKind,
        Result,
    },
    messages::{
        PortType,
        TransactionId,
    },
    response_future::ResponseFuture,
    responses::{
        FindNodeResponse,
        GetPeersResponse,
        NodeIDResponse,
    },
};
use failure::ResultExt;
use futures::lock::Mutex;
use krpc_encoding::{
    self as proto,
    Envelope,
    Message,
    NodeID,
    Query,
};
use rand;
use std::{
    self,
    net::SocketAddr,
};
use tokio::net::udp::split::UdpSocketSendHalf;

pub struct SendTransport {
    socket: Mutex<UdpSocketSendHalf>,
    transactions: ActiveTransactions,
}

impl SendTransport {
    pub fn new(socket: UdpSocketSendHalf, transactions: ActiveTransactions) -> SendTransport {
        SendTransport {
            socket: Mutex::new(socket),
            transactions,
        }
    }

    pub async fn ping(&self, id: NodeID, address: SocketAddr) -> Result<NodeID> {
        let response = self.request(address, Query::Ping { id }).await?;

        Ok(NodeIDResponse::from_response(response)?)
    }

    pub async fn find_node(
        &self,
        id: NodeID,
        address: SocketAddr,
        target: NodeID,
    ) -> Result<FindNodeResponse> {
        let response = self
            .request(address, Query::FindNode { id, target })
            .await?;

        Ok(FindNodeResponse::from_response(response)?)
    }

    pub async fn get_peers(
        &self,
        id: NodeID,
        address: SocketAddr,
        info_hash: NodeID,
    ) -> Result<GetPeersResponse> {
        let response = self
            .request(address, Query::GetPeers { id, info_hash })
            .await?;

        Ok(GetPeersResponse::from_response(response)?)
    }

    pub async fn announce_peer(
        &self,
        id: NodeID,
        token: Vec<u8>,
        address: SocketAddr,
        info_hash: NodeID,
        port_type: PortType,
    ) -> Result<NodeID> {
        let (port, implied_port) = match port_type {
            PortType::Implied => (None, true),
            PortType::Port(port) => (Some(port), false),
        };

        let response = self
            .request(
                address,
                Query::AnnouncePeer {
                    id,
                    token,
                    info_hash,
                    port,
                    implied_port,
                },
            )
            .await?;

        Ok(NodeIDResponse::from_response(response)?)
    }

    pub async fn send(&self, address: SocketAddr, message: Envelope) -> Result<()> {
        let encoded = message
            .encode()
            .map_err(|cause| ErrorKind::SendEncodingError { cause })?;

        let mut socket = self.socket.lock().await;

        socket
            .send_to(&encoded, &address)
            .await
            .with_context(|_| ErrorKind::SendError { to: address })?;

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
