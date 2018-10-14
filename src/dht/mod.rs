use errors::{Error, Result};
use proto::NodeID;
use routing::RoutingTable;
use transport::{PortType, RecvTransport, SendTransport};

use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};

use tokio::prelude::*;

mod handler;

/// BitTorrent DHT node
#[derive(Clone)]
pub struct Dht {
    id: NodeID,
    torrents: Arc<Mutex<HashMap<NodeID, Vec<SocketAddrV4>>>>,
    send_transport: Arc<SendTransport>,
    routing_table: Arc<Mutex<RoutingTable>>,
    // TODO: Add Routing Table When Stabilized
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
        bootstrap_node: SocketAddr,
    ) -> impl Future<Item = (), Error = Error> {
        // TODO:
        // * Add Node
        // * Query Node for Self Until Some Amount of Nodes Have Been Successfully Added
        future::ok(())
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
