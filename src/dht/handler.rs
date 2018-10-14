use dht::Dht;
use errors::{Error, ErrorKind, Result};
use proto::{Addr, Message, MessageType, NodeID, Query, Response};
use routing::{FindNodeResult, RoutingTable};
use transport::Request;

use std::net::{SocketAddr, SocketAddrV4};
use std::ops::DerefMut;

use tokio::prelude::*;

impl Dht {
    pub fn handle_requests<S: Stream<Item = (Request, SocketAddr), Error = Error>>(
        self,
        stream: S,
    ) -> impl Future<Item = (), Error = Error> {
        stream
            .and_then(move |(request, from)| -> Result<()> {
                let response = self.handle_request(request, as_v4_addr(from)?);
                self.send_transport.send(from, response)
            }).filter_map(|_| -> Option<()> { None })
            .into_future()
            .map(|_| ())
            .map_err(|next| next.0)
    }

    fn handle_request(&self, request: Request, from: SocketAddrV4) -> Message {
        let result = match request.query {
            Query::Ping { id } => self.handle_ping(from, id),
            Query::FindNode { id, target } => self.handle_find_node(from, id, target),
            Query::GetPeers { id, info_hash } => self.handle_get_peers(from, id, info_hash),
            Query::AnnouncePeer {
                id,
                implied_port,
                port,
                info_hash,
                token,
            } => self.handle_announce_peer(from, id, implied_port, port, info_hash, token),
            _ => Err(ErrorKind::UnimplementedRequestType.into()),
        };

        let message_type = match result {
            Ok(response) => MessageType::Response { response },
            Err(err) => MessageType::Error {
                error: err.as_request_error(),
            },
        };

        Message {
            ip: None,
            transaction_id: request.transaction_id,
            version: None,
            message_type,
        }
    }

    fn handle_ping(&self, from: SocketAddrV4, id: NodeID) -> Result<Response> {
        let mut routing_table = self.routing_table.lock()?;
        record_request(&mut routing_table, id, from)?;

        Ok(Response::OnlyId {
            id: self.id.clone(),
        })
    }

    fn handle_find_node(&self, from: SocketAddrV4, id: NodeID, target: NodeID) -> Result<Response> {
        let mut routing_table = self.routing_table.lock()?;
        record_request(&mut routing_table, id, from)?;

        let nodes = match routing_table.find_node(&target) {
            FindNodeResult::Node(node) => vec![node],
            FindNodeResult::Nodes(nodes) => nodes,
        };

        Ok(Response::NextHop {
            id: self.id.clone(),
            token: None,
            nodes,
        })
    }

    fn handle_get_peers(
        &self,
        from: SocketAddrV4,
        id: NodeID,
        info_hash: NodeID,
    ) -> Result<Response> {
        let mut routing_table = self.routing_table.lock()?;
        record_request(&mut routing_table, id, from)?;

        let token_bytes = routing_table.generate_token(&from).to_vec();
        let token = Some(token_bytes);
        let torrents = self.torrents.lock()?;
        let torrent = torrents.get(&info_hash);

        if let Some(peers) = torrent {
            Ok(Response::GetPeers {
                id: self.id.clone(),
                token,
                peers: peers.iter().map(|peer| Addr::from(peer.clone())).collect(),
            })
        } else {
            let nodes = routing_table.find_nodes(&info_hash);

            Ok(Response::NextHop {
                id: self.id.clone(),
                token,
                nodes,
            })
        }
    }

    fn handle_announce_peer(
        &self,
        mut from: SocketAddrV4,
        id: NodeID,
        implied_port: u8,
        port: Option<u16>,
        info_hash: NodeID,
        token: Vec<u8>,
    ) -> Result<Response> {
        let mut routing_table = self.routing_table.lock()?;

        if !routing_table.verify_token(&token, &from) {
            return Err(ErrorKind::InvalidToken)?;
        };

        let addr = if implied_port == 1 {
            from
        } else {
            let actual_port = match port {
                None => return Err(ErrorKind::InsufficientAddress)?,
                Some(port) => port,
            };

            from.set_port(actual_port);
            from
        };

        record_request(&mut routing_table, id, from)?;

        // TODO: Duplicates
        let mut torrents = self.torrents.lock()?;

        torrents
            .entry(info_hash)
            .or_insert_with(Vec::new)
            .push(addr);

        Ok(Response::OnlyId {
            id: self.id.clone(),
        })
    }
}

fn as_v4_addr(addr: SocketAddr) -> Result<SocketAddrV4> {
    match addr {
        SocketAddr::V4(addr) => Ok(addr),
        SocketAddr::V6(..) => Err(ErrorKind::UnsupportedAddressTypeError)?,
    }
}

fn record_request<T: DerefMut<Target = RoutingTable>>(
    routing_table: &mut T,
    id: NodeID,
    from: SocketAddrV4,
) -> Result<()> {
    routing_table
        .deref_mut()
        .get_or_add(id, from)
        .map(|node| node.mark_successful_request_from());

    Ok(())
}
