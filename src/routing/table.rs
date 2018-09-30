use chrono::NaiveDateTime;
use proto::{NodeID, NodeInfo};
use std::net::SocketAddrV4;

struct Node {
    id: NodeID,
    address: SocketAddrV4,

    /// Token received when discovering this node. Used when announcing ourselves to this peer.
    announce_token: Vec<u8>,

    /// Token which should be presented to us when announcing a peer. Updated every 10 minutes.
    token: u64,

    /// Last valid token which could also be presented when a peer announces. Updated every 10
    /// minutes.
    last_token: u64,

    /// Last time this node was contacted successfully.
    last_contacted: NaiveDateTime,
}

enum NodeState {
    /// A good node is a node has responded to one of our queries within the last 15 minutes. A node
    /// is also good if it has ever responded to one of our queries and has sent us a query within
    /// the last 15 minutes.
    Good,

    /// After 15 minutes of inactivity, a node becomes questionable.
    Questionable,

    /// Nodes become bad when they fail to respond to multiple queries in a row. At this point, they
    /// are removed from the routing table.
    Bad { failed_requests: u8 },
}

enum FindNodeResult {
    Node(NodeInfo),
    Nodes(Vec<NodeInfo>),
}

struct RoutingTable;

impl RoutingTable {
    /// Adds a node to the routing table. The node is assumed to be good until proven bad.
    fn add_node(info: NodeInfo) {}

    /// Finds the node with `id`, or about the `k` nearest good nodes to the `id` if the exact node
    /// couldn't be found. More or less than `k` nodes may be returned.
    fn find_node(id: NodeID, k: u8) -> FindNodeResult {
        FindNodeResult::Nodes(Vec::new())
    }

    /// Finds the around the `k` nearest nodes to `id`. More or less than `k` nodes may be returned.
    fn find_nodes(id: NodeID, k: u8) -> Vec<NodeInfo> {
        Vec::new()
    }

    /// Returns true if `token` is valid for `id` to announce that it is downloading a torrent.
    fn verify_token(id: NodeID, token: Vec<u8>) -> bool {
        false
    }
}
