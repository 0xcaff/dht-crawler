use crate::{
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    proto::{
        Message,
        NodeID,
        Query,
    },
    transport::{
        active_transactions::ActiveTransactions,
        messages::{
            FindNodeResponse,
            GetPeersResponse,
            NodeIDResponse,
            PortType,
            Request,
            Response,
            TransactionId,
        },
        response_future::ResponseFuture,
    },
};
use byteorder::NetworkEndian;
use bytes::ByteOrder;
use failure::ResultExt;
use rand;
use std::{
    self,
    net::SocketAddr,
};
use tokio::prelude::*;

pub struct SendTransport {
    socket: std::net::UdpSocket,
    transactions: ActiveTransactions,
    read_only: bool,
}

impl SendTransport {
    pub fn new(
        socket: std::net::UdpSocket,
        transactions: ActiveTransactions,
        read_only: bool,
    ) -> SendTransport {
        SendTransport {
            socket,
            transactions,
            read_only,
        }
    }

    pub fn request(
        &self,
        address: SocketAddr,
        transaction_id: TransactionId,
        request: Request,
    ) -> impl Future<Item = Response, Error = Error> {
        let transaction_future_result =
            ResponseFuture::wait_for_tx(transaction_id, self.transactions.clone());

        self.send_request(address, transaction_id, request)
            .into_future()
            .and_then(move |_| transaction_future_result)
            .and_then(|fut| fut)
            .and_then(|envelope| Response::from(envelope))
    }

    /// Adds `transaction_id` to the request and sends it.
    fn send_request(
        &self,
        address: SocketAddr,
        transaction_id: TransactionId,
        mut request: Request,
    ) -> Result<()> {
        let mut buf = [0u8; 4];
        NetworkEndian::write_u32(&mut buf, transaction_id);
        request.transaction_id.extend_from_slice(&buf);

        Ok(self.send(address, request.into())?)
    }

    /// Synchronously sends a request to `address`.
    ///
    /// The sending is done synchronously because doing it asynchronously was
    /// cumbersome and didn't make anything faster. UDP sending rarely
    /// blocks.
    pub fn send(&self, address: SocketAddr, message: Message) -> Result<()> {
        self.socket
            .send_to(&message.encode()?, &address)
            .with_context(|_| ErrorKind::SendError { to: address })?;

        Ok(())
    }

    fn get_transaction_id() -> TransactionId {
        rand::random::<TransactionId>()
    }

    fn build_request(&self, query: Query) -> Request {
        Request {
            transaction_id: Vec::new(),
            version: None,
            query,
            read_only: self.read_only,
        }
    }

    pub fn ping(
        &self,
        id: NodeID,
        address: SocketAddr,
    ) -> impl Future<Item = NodeID, Error = Error> {
        self.request(
            address,
            Self::get_transaction_id(),
            self.build_request(Query::Ping { id }),
        )
        .and_then(NodeIDResponse::from_response)
    }

    pub fn find_node(
        &self,
        id: NodeID,
        address: SocketAddr,
        target: NodeID,
    ) -> impl Future<Item = FindNodeResponse, Error = Error> {
        self.request(
            address,
            Self::get_transaction_id(),
            self.build_request(Query::FindNode { id, target }),
        )
        .and_then(FindNodeResponse::from_response)
    }

    pub fn get_peers(
        &self,
        id: NodeID,
        address: SocketAddr,
        info_hash: NodeID,
    ) -> impl Future<Item = GetPeersResponse, Error = Error> {
        self.request(
            address,
            Self::get_transaction_id(),
            self.build_request(Query::GetPeers { id, info_hash }),
        )
        .and_then(GetPeersResponse::from_response)
    }

    pub fn announce_peer(
        &self,
        id: NodeID,
        token: Vec<u8>,
        address: SocketAddr,
        info_hash: NodeID,
        port_type: PortType,
    ) -> impl Future<Item = NodeID, Error = Error> {
        let (port, implied_port) = match port_type {
            PortType::Implied => (None, true),
            PortType::Port(port) => (Some(port), false),
        };

        self.request(
            address,
            Self::get_transaction_id(),
            self.build_request(Query::AnnouncePeer {
                id,
                token,
                info_hash,
                port,
                implied_port,
            }),
        )
        .and_then(NodeIDResponse::from_response)
    }
}
