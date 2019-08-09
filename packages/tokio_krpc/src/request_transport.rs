use crate::{
    responses::{
        FindNodeResponse,
        GetPeersResponse,
        NodeIDResponse,
    },
    send_errors::Result,
    PortType,
    SendTransport,
};
use krpc_encoding::{
    NodeID,
    Query,
};
use std::net::SocketAddrV4;
use std::borrow::Borrow;

/// High level wrapper around a UDP socket for sending typed queries and
/// receiving typed responses.
pub struct RequestTransport {
    send_transport: Box<dyn Borrow<SendTransport>>,
}

impl RequestTransport {
    pub fn new<T: Borrow<SendTransport> + 'static>(send_transport: T) -> RequestTransport {
        RequestTransport {
            send_transport: Box::new(send_transport),
        }
    }

    pub async fn ping(&self, id: NodeID, address: SocketAddrV4) -> Result<NodeID> {
        let response = (*self.send_transport)
            .borrow()
            .request(address.into(), Query::Ping { id })
            .await?;

        Ok(NodeIDResponse::from_response(response)?)
    }

    pub async fn find_node(
        &self,
        id: NodeID,
        address: SocketAddrV4,
        target: NodeID,
    ) -> Result<FindNodeResponse> {
        let response = (*self.send_transport)
            .borrow()
            .request(address.into(), Query::FindNode { id, target })
            .await?;

        Ok(FindNodeResponse::from_response(response)?)
    }

    pub async fn get_peers(
        &self,
        id: NodeID,
        address: SocketAddrV4,
        info_hash: NodeID,
    ) -> Result<GetPeersResponse> {
        let response = (*self.send_transport)
            .borrow()
            .request(address.into(), Query::GetPeers { id, info_hash })
            .await?;

        Ok(GetPeersResponse::from_response(response)?)
    }

    pub async fn announce_peer(
        &self,
        id: NodeID,
        token: Vec<u8>,
        address: SocketAddrV4,
        info_hash: NodeID,
        port_type: PortType,
    ) -> Result<NodeID> {
        let (port, implied_port) = match port_type {
            PortType::Implied => (None, true),
            PortType::Port(port) => (Some(port), false),
        };

        let response = (*self.send_transport)
            .borrow()
            .request(
                address.into(),
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
}
