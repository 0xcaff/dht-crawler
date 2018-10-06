use errors::{Error, Result};
use proto::NodeID;
use transport::{PortType, Transport};

use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};

use tokio::prelude::*;

/// BitTorrent DHT node
pub struct Dht {
    id: NodeID,
    torrents: HashMap<NodeID, Vec<SocketAddrV4>>,
    transport: Transport,
    // TODO: Add Routing Table When Stabilized
}

impl Dht {
    pub fn new(bind_addr: SocketAddr) -> Result<Dht> {
        let id = NodeID::random();
        let torrents = HashMap::new();
        let transport = Transport::new(bind_addr)?;

        Ok(Dht {
            id,
            torrents,
            transport,
        })
    }

    /// Start handling inbound messages from other peers in the network. Continues to handle while
    /// the future is polled. This should be called once for each instance of the DHT before calling
    /// other functions.
    pub fn start(&self) -> impl Future<Item = (), Error = Error> {
        future::ok(())
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
