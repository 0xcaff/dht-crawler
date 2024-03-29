use crate::{
    errors::{
        ErrorKind,
        Result,
    },
    routing::{
        Node,
        RoutingTable,
    },
};
use futures::future;
use futures_util::TryStreamExt;
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use std::{
    collections::HashMap,
    net::{
        SocketAddr,
        SocketAddrV4,
    },
    pin::Pin,
    sync::{
        Arc,
        Mutex,
    },
};
use tokio::net::UdpSocket;
use tokio_krpc::{
    KRPCNode,
    PortType,
    RequestTransport,
    SendTransport,
};

mod handler;

/// BitTorrent DHT node
#[derive(Clone)]
pub struct Dht {
    id: NodeID,
    torrents: Arc<Mutex<HashMap<NodeID, Vec<SocketAddrV4>>>>,
    request_transport: Arc<RequestTransport>,
    send_transport: Arc<SendTransport>,
    routing_table: Arc<Mutex<RoutingTable>>,
}

impl Dht {
    // todo: why the mutex and arc everwhere?

    /// Start handling inbound messages from other peers in the network.
    /// Continues to handle while the future is polled.
    pub async fn start(bind_addr: SocketAddr) -> Result<(Dht, impl future::Future<Output = ()>)> {
        let socket = UdpSocket::bind(&bind_addr)
            .await
            .map_err(|cause| ErrorKind::BindError { cause })?;
        let transport = KRPCNode::new(socket);
        let (send_transport, request_stream) = transport.serve();

        let id = NodeID::random();
        let torrents = HashMap::new();
        let routing_table = RoutingTable::new(id.clone());
        let send_transport_arc = Arc::new(send_transport);

        let dht = Dht {
            id: id.clone(),
            torrents: Arc::new(Mutex::new(torrents)),
            request_transport: Arc::new(RequestTransport::new(id, send_transport_arc.clone())),
            send_transport: send_transport_arc,
            routing_table: Arc::new(Mutex::new(routing_table)),
        };

        Ok((dht.clone(), dht.handle_requests(request_stream.err_into())))
    }

    /// Bootstraps the routing table by finding nodes near our node id and
    /// adding them to the routing table.
    pub async fn bootstrap_routing_table(&self, addrs: Vec<SocketAddrV4>) -> Result<()> {
        let send_transport = self.request_transport.clone();
        let routing_table_arc = self.routing_table.clone();
        let id = self.id.clone();

        future::join_all(addrs.into_iter().map(move |addr| {
            Self::discover_nodes_of(
                addr,
                id.clone(),
                send_transport.clone(),
                routing_table_arc.clone(),
            )
        }))
        .await;

        Ok(())
    }

    async fn discover_nodes_of(
        addr: SocketAddrV4,
        self_id: NodeID,
        request_transport: Arc<RequestTransport>,
        routing_table_arc: Arc<Mutex<RoutingTable>>,
    ) -> Result<()> {
        // todo: weird recursive thing
        // todo: populate routing table

        let response = request_transport
            .find_node(addr.clone().into(), self_id.clone())
            // todo: standardize timeout
            .await?;

        let mut node = Node::new(response.id, addr.into());
        node.mark_successful_request();

        {
            let mut routing_table = routing_table_arc.lock()?;
            routing_table.add_node(node);
        }

        let f: Pin<Box<dyn future::Future<Output = _>>> =
            Box::pin(future::join_all(response.nodes.into_iter().map(|node| {
                Self::discover_neighbors_of(
                    node,
                    self_id.clone(),
                    request_transport.clone(),
                    routing_table_arc.clone(),
                )
            })));

        f.await;

        Ok(())
    }

    async fn discover_neighbors_of(
        node: NodeInfo,
        self_id: NodeID,
        request_transport: Arc<RequestTransport>,
        routing_table_arc: Arc<Mutex<RoutingTable>>,
    ) {
        Self::discover_nodes_of(node.address, self_id, request_transport, routing_table_arc)
            .await
            .unwrap_or_else(|e| eprintln!("Error While Bootstrapping {}", e));
    }

    /// Gets a list of peers seeding `info_hash`.
    pub async fn get_peers(&self, _info_hash: NodeID) -> Result<Vec<SocketAddrV4>> {
        // TODO:
        // * Return From torrents Table if Exists
        // * Fetch By Calling get_nodes otherwise
        unimplemented!()
    }

    /// Announces that we have information about an info_hash on `port`.
    pub async fn announce(&self, _info_hash: NodeID, _port: PortType) -> Result<()> {
        // TODO:
        // * Send Announce to all Peers With Tokens
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        addr::{
            AsV4Address,
            IntoSocketAddr,
        },
        errors::Error as DhtError,
        Dht,
    };
    use failure::Error;
    use tokio::{
        spawn,
        task::spawn_local,
    };

    #[tokio::test]
    #[ignore]
    async fn test_bootstrap() -> Result<(), Error> {
        let addr = "0.0.0.0:23170".into_addr();
        let (dht, dht_future) = Dht::start(addr).await?;

        let bootstrap_future = dht.bootstrap_routing_table(vec![
            "router.utorrent.com:6881".into_addr().into_v4()?,
            "router.bittorrent.com:6881".into_addr().into_v4()?,
        ]);

        spawn_local(dht_future);
        bootstrap_future.await?;

        let routing_table = dht.routing_table.lock().map_err(DhtError::from)?;

        assert!(routing_table.len() > 0);

        Ok(())
    }
}
