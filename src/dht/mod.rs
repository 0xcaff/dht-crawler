use crate::{
    errors::{
        Error,
        Result,
    },
    proto::{
        NodeID,
        NodeInfo,
    },
    routing::{
        Node,
        RoutingTable,
    },
    transport::{
        PortType,
        RecvTransport,
        SendTransport,
    },
};
use futuresx::future as futurex;
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
use tokio::prelude::*;

mod handler;

/// BitTorrent DHT node
#[derive(Clone)]
pub struct Dht {
    id: NodeID,
    torrents: Arc<Mutex<HashMap<NodeID, Vec<SocketAddrV4>>>>,
    send_transport: Arc<SendTransport>,
    routing_table: Arc<Mutex<RoutingTable>>,
}

impl Dht {
    /// Start handling inbound messages from other peers in the network.
    /// Continues to handle while the future is polled.
    pub fn start(bind_addr: SocketAddr) -> Result<(Dht, impl futurex::Future<Output = ()>)> {
        let transport = RecvTransport::new(bind_addr)?;
        let (send_transport, request_stream) = transport.serve();

        let id = NodeID::random();
        let torrents = Arc::new(Mutex::new(HashMap::new()));
        let routing_table = Arc::new(Mutex::new(RoutingTable::new(id.clone())));

        let dht = Dht {
            id,
            torrents,
            send_transport: Arc::new(send_transport),
            routing_table,
        };

        Ok((dht.clone(), dht.handle_requests(request_stream)))
    }

    /// Bootstraps the routing table by finding nodes near our node id and
    /// adding them to the routing table.
    pub async fn bootstrap_routing_table(&self, addrs: Vec<SocketAddrV4>) -> Result<()> {
        let send_transport = self.send_transport.clone();
        let routing_table_arc = self.routing_table.clone();
        let id = self.id.clone();

        futurex::join_all(addrs.into_iter().map(move |addr| {
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
        send_transport: Arc<SendTransport>,
        routing_table_arc: Arc<Mutex<RoutingTable>>,
    ) -> Result<()> {
        let response = send_transport
            .find_node(self_id.clone(), addr.clone().into(), self_id.clone())
            .await?;

        let mut node = Node::new(response.id, addr.into());
        node.mark_successful_request();

        {
            let mut routing_table = routing_table_arc.lock()?;
            routing_table.add_node(node);
        }

        let f: Pin<Box<dyn futurex::Future<Output = _>>> =
            Box::pin(futurex::join_all(response.nodes.into_iter().map(|node| {
                Self::discover_neighbors_of(
                    node,
                    self_id.clone(),
                    send_transport.clone(),
                    routing_table_arc.clone(),
                )
            })));

        f.await;

        Ok(())
    }

    async fn discover_neighbors_of(
        node: NodeInfo,
        self_id: NodeID,
        send_transport: Arc<SendTransport>,
        routing_table_arc: Arc<Mutex<RoutingTable>>,
    ) {
        Self::discover_nodes_of(node.address, self_id, send_transport, routing_table_arc)
            .await
            .unwrap_or_else(|e| eprintln!("Error While Bootstrapping {}", e));
    }

    /// Gets a list of peers seeding `info_hash`.
    pub fn get_peers(
        &self,
        _info_hash: NodeID,
    ) -> impl Future<Item = Vec<SocketAddrV4>, Error = Error> {
        // TODO:
        // * Return From torrents Table if Exists
        // * Fetch By Calling get_nodes otherwise
        future::ok(Vec::new())
    }

    /// Announces that we have information about an info_hash on `port`.
    pub fn announce(
        &self,
        _info_hash: NodeID,
        _port: PortType,
    ) -> impl Future<Item = (), Error = Error> {
        // TODO:
        // * Send Announce to all Peers With Tokens
        future::ok(())
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
    use futuresx_util::{
        future::FutureExt as FutureXExt,
        try_future::TryFutureExt,
    };
    use tokio::runtime::current_thread::Runtime;

    #[test]
    #[ignore]
    fn test_bootstrap() -> Result<(), Error> {
        let addr = "0.0.0.0:23170".into_addr();
        let (dht, dht_future) = Dht::start(addr)?;

        let bootstrap_future = dht.bootstrap_routing_table(vec![
            "router.utorrent.com:6881".into_addr().into_v4()?,
            "router.bittorrent.com:6881".into_addr().into_v4()?,
        ]);

        let mut runtime = Runtime::new()?;
        runtime.spawn(dht_future.unit_error().boxed().compat());
        runtime.block_on(bootstrap_future.boxed_local().compat())?;

        let routing_table = dht.routing_table.lock().map_err(DhtError::from)?;

        assert!(routing_table.len() > 0);

        Ok(())
    }
}
