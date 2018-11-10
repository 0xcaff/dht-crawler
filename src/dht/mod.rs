use errors::{Error, Result};

use proto::NodeID;
use routing::{Node, RoutingTable};
use transport::{PortType, RecvTransport, SendTransport};

use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};

use std::time::Duration;
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
    /// Start handling inbound messages from other peers in the network. Continues to handle while
    /// the future is polled.
    pub fn start(bind_addr: SocketAddr) -> Result<(Dht, impl Future<Item = (), Error = Error>)> {
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

    /// Bootstraps the routing table by finding nodes near our node id and adding them to the
    /// routing table.
    pub fn bootstrap_routing_table(
        &self,
        addrs: Vec<SocketAddrV4>,
    ) -> impl Future<Item = (), Error = Error> {
        let send_transport = self.send_transport.clone();
        let routing_table_arc = self.routing_table.clone();
        let id = self.id.clone();

        let bootstrap_futures = addrs.into_iter().map(move |addr| {
            Self::discover_nodes_of(
                addr,
                id.clone(),
                send_transport.clone(),
                routing_table_arc.clone(),
            )
        });

        let bootstrap_future = future::join_all(bootstrap_futures).and_then(|_| Ok(()));

        bootstrap_future
    }

    fn discover_nodes_of(
        addr: SocketAddrV4,
        self_id: NodeID,
        send_transport: Arc<SendTransport>,
        routing_table_arc: Arc<Mutex<RoutingTable>>,
    ) -> Box<Future<Item = (), Error = Error> + Send> {
        let cloned_routing_table = routing_table_arc.clone();

        let fut = send_transport
            .find_node(self_id.clone(), addr.clone().into(), self_id.clone())
            .timeout(Duration::from_secs(5))
            .map_err(Error::from)
            .and_then(move |response| {
                let mut node = Node::new(response.id, addr.into());
                node.mark_successful_request();

                let mut routing_table = routing_table_arc.lock()?;
                routing_table.add_node(node);

                Ok(response.nodes)
            }).and_then(move |nodes| {
                let cloned_send_transport = send_transport.clone();
                let cloned_self_id = self_id;

                future::join_all(nodes.into_iter().map(move |node| {
                    Self::discover_nodes_of(
                        node.address,
                        cloned_self_id.clone(),
                        cloned_send_transport.clone(),
                        cloned_routing_table.clone(),
                    ).or_else(|e| {
                        println!("Received Error While Bootstrapping {}", e);
                        Ok(())
                    })
                })).and_then(|_| Ok(()))
            });

        Box::new(fut)
    }

    /// Gets a list of peers seeding `info_hash`.
    pub fn get_peers(
        &self,
        info_hash: NodeID,
    ) -> impl Future<Item = Vec<SocketAddrV4>, Error = Error> {
        // TODO:
        // * Return From torrents Table if Exists
        // * Fetch By Calling get_nodes otherwise
        future::ok(Vec::new())
    }

    /// Announces that we have information about an info_hash on `port`.
    pub fn announce(
        &self,
        info_hash: NodeID,
        port: PortType,
    ) -> impl Future<Item = (), Error = Error> {
        // TODO:
        // * Send Announce to all Peers With Tokens
        future::ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;
    use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};
    use tokio::runtime::Runtime;
    use Dht;

    fn flatten_addrs<I, A>(nodes: Vec<A>) -> Vec<SocketAddrV4>
    where
        I: Iterator<Item = SocketAddr>,
        A: ToSocketAddrs<Iter = I>,
    {
        nodes
            .into_iter()
            // TODO: Remove .unwrap
            .flat_map(|addr| addr.to_socket_addrs().unwrap())
            .filter_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                _ => None,
            })
            .collect()
    }

    #[test]
    #[ignore]
    fn test_bootstrap() {
        let addr = "0.0.0.0:23170".to_socket_addrs().unwrap().nth(0).unwrap();
        let (dht, dht_future) = Dht::start(addr).unwrap();

        let bootstrap_future = dht.bootstrap_routing_table(flatten_addrs(vec![
            "router.utorrent.com:6881",
            "router.bittorrent.com:6881",
        ]));

        let mut runtime = Runtime::new().unwrap();
        runtime.spawn(dht_future.map_err(|e| println!("{}", e)));
        runtime.block_on(bootstrap_future).unwrap();

        let routing_table = dht.routing_table.lock().unwrap();

        assert!(routing_table.len() > 0);
    }
}
