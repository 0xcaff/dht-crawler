use crate::{
    errors::{
        Error,
        Result,
    },
    proto::NodeID,
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
use std::{
    collections::HashMap,
    net::{
        SocketAddr,
        SocketAddrV4,
    },
    sync::{
        Arc,
        Mutex,
    },
    time::Duration,
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
    pub fn start(bind_addr: SocketAddr) -> Result<(Dht, impl Future<Item = (), Error = ()>)> {
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

        let bootstrap_future = future::join_all(bootstrap_futures).map(|_| ());

        bootstrap_future
    }

    fn discover_nodes_of(
        addr: SocketAddrV4,
        self_id: NodeID,
        send_transport: Arc<SendTransport>,
        routing_table_arc: Arc<Mutex<RoutingTable>>,
    ) -> Box<dyn Future<Item = (), Error = Error> + Send> {
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
            })
            .and_then(move |nodes| {
                let cloned_send_transport = send_transport.clone();
                let cloned_self_id = self_id;

                future::join_all(nodes.into_iter().map(move |node| {
                    Self::discover_nodes_of(
                        node.address,
                        cloned_self_id.clone(),
                        cloned_send_transport.clone(),
                        cloned_routing_table.clone(),
                    )
                    .or_else(|e| {
                        eprintln!("Error While Bootstrapping {}", e);
                        Ok(())
                    })
                }))
                .map(|_| ())
            });

        Box::new(fut)
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
        proto::NodeID,
        stream::{
            run_forever,
            select_all,
        },
        transport::{
            RecvTransport,
            SendTransport,
        },
        Dht,
    };
    use failure::Error;
    use futures::Stream;
    use std::{
        iter,
        net::SocketAddrV4,
        sync::Arc,
        time::{
            Duration,
            Instant,
        },
    };
    use tokio::{
        prelude::*,
        runtime::Runtime,
    };

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
        runtime.spawn(dht_future);
        runtime.block_on(bootstrap_future)?;

        let routing_table = dht.routing_table.lock().map_err(DhtError::from)?;

        assert!(routing_table.len() > 0);

        Ok(())
    }

    #[derive(Debug)]
    struct Node {
        id: NodeID,
        address: SocketAddrV4,
        time_discovered: Instant,
    }

    impl Node {
        pub fn new(id: NodeID, address: SocketAddrV4) -> Node {
            Node {
                id,
                address,
                time_discovered: Instant::now(),
            }
        }
    }

    #[test]
    #[ignore]
    fn test_traversal() -> Result<(), Error> {
        let node_id = NodeID::random();
        let bind_addr = "0.0.0.0:21130".into_addr();
        let bootstrap_addr = "router.bittorrent.com:6881".into_addr().into_v4()?;

        let transport = RecvTransport::new(bind_addr)?;
        let (send_transport, request_stream) = transport.serve_read_only();

        let send_transport_arc = Arc::new(send_transport);

        let mut runtime = Runtime::new()?;
        runtime.spawn(run_forever(request_stream.map(|_| ()).or_else(|err| {
            eprintln!("Error While Handling Requests: {}", err);

            Ok(())
        })));

        runtime
            .block_on(run_forever(
                traverse(node_id, bootstrap_addr, send_transport_arc)
                    .map(|node| println!("Node Discovered: {:#?}", node))
                    .or_else(|e| {
                        eprintln!("Error While Traversing: {}", e);
                        Ok(())
                    }),
            ))
            .ok();

        Ok(())
    }

    fn traverse(
        self_id: NodeID,
        addr: SocketAddrV4,
        send_transport: Arc<SendTransport>,
    ) -> Box<dyn Stream<Item = Node, Error = DhtError> + Send> {
        Box::new(
            send_transport
                .find_node(self_id.clone(), addr.clone().into(), self_id.clone())
                .timeout(Duration::from_secs(5))
                .map_err(DhtError::from)
                .map(move |response| {
                    let node = Node::new(response.id, addr);
                    let result_stream = Box::new(stream::once(Ok(node)))
                        as Box<dyn Stream<Item = Node, Error = DhtError> + Send>;

                    select_all(
                        iter::once(result_stream).chain(response.nodes.into_iter().map(|node| {
                            traverse(self_id.clone(), node.address, send_transport.clone())
                        })),
                    )
                })
                .into_stream()
                .flatten(),
        )
    }
}
