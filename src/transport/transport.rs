use errors::{Error, ErrorKind, Result};
use failure::ResultExt;

use proto::{NodeID, Query};

use rand;

use std;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use byteorder::{NetworkEndian, WriteBytesExt};

use tokio;
use tokio::prelude::*;
use tokio::reactor::Handle;

use proto::MessageType;
use transport::inbound::InboundMessageStream;
use transport::messages::{
    FindNodeResponse, GetPeersResponse, NodeIDResponse, PortType, Request, Response, TransactionId,
};
use transport::response::{ResponseFuture, TransactionMap};

pub struct Transport {
    /// Socket used for sending messages
    send_socket: std::net::UdpSocket,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl Transport {
    pub fn new(bind_address: SocketAddr) -> Result<Transport> {
        let send_socket = std::net::UdpSocket::bind(&bind_address).context(ErrorKind::BindError)?;

        Ok(Transport {
            send_socket,
            transactions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn make_recv_socket(&self) -> Result<tokio::net::UdpSocket> {
        let raw_recv_socket = self.send_socket.try_clone().context(ErrorKind::BindError)?;
        let recv_socket = tokio::net::UdpSocket::from_std(raw_recv_socket, &Handle::default())
            .context(ErrorKind::BindError)?;

        Ok(recv_socket)
    }

    pub fn handle_inbound(&self) -> impl Stream<Item = (Request, SocketAddr), Error = Error> {
        let transactions = self.transactions.clone();

        self.make_recv_socket()
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
            .filter_map(|m| m)
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

    /// Synchronously sends a request to `address`.
    ///
    /// The sending is done synchronously because doing it asynchronously was cumbersome and didn't
    /// make anything faster. UDP sending rarely blocks.
    fn send_request(
        &self,
        address: SocketAddr,
        transaction_id: TransactionId,
        mut request: Request,
    ) -> Result<()> {
        request
            .transaction_id
            .write_u32::<NetworkEndian>(transaction_id)
            .with_context(|_| ErrorKind::SendError { to: address })?;

        let encoded = request.encode()?;

        self.send_socket
            .send_to(&encoded, &address)
            .with_context(|_| ErrorKind::SendError { to: address })?;

        Ok(())
    }

    fn get_transaction_id() -> TransactionId {
        rand::random::<TransactionId>()
    }

    fn build_request(query: Query) -> Request {
        Request {
            transaction_id: Vec::new(),
            version: None,
            query,
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
            Self::build_request(Query::Ping { id }),
        ).and_then(NodeIDResponse::from_response)
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
            Self::build_request(Query::FindNode { id, target }),
        ).and_then(FindNodeResponse::from_response)
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
            Self::build_request(Query::GetPeers { id, info_hash }),
        ).and_then(GetPeersResponse::from_response)
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
            PortType::Implied => (None, 1),
            PortType::Port(port) => (Some(port), 0),
        };

        self.request(
            address,
            Self::get_transaction_id(),
            Self::build_request(Query::AnnouncePeer {
                id,
                token,
                info_hash,
                port,
                implied_port,
            }),
        ).and_then(NodeIDResponse::from_response)
    }
}
