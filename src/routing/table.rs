use std::cmp;
use std::mem;
use std::net::SocketAddrV4;
use std::ops::Deref;

use chrono::{NaiveDate, NaiveDateTime, Utc};
use proto::{NodeID, NodeInfo};
use rand;

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

    /// Last time a message was sent from ourselves to this node and a response was received
    /// successfully.
    last_request_to: NaiveDateTime,

    /// Last time a valid request was received from this node.
    last_request_from: NaiveDateTime,

    /// Number of failed requests from us to the node since `last_request_to`.
    failed_requests: u8,
}

impl<'a> Into<NodeInfo> for &'a Node {
    fn into(self) -> NodeInfo {
        NodeInfo::new(self.id.clone(), self.address)
    }
}

impl Into<NodeInfo> for Node {
    fn into(self) -> NodeInfo {
        NodeInfo::new(self.id, self.address)
    }
}

#[derive(PartialEq)]
enum NodeState {
    /// A good node is a node has responded to one of our queries within the last 15 minutes. A node
    /// is also good if it has ever responded to one of our queries and has sent us a query within
    /// the last 15 minutes.
    Good,

    /// After 15 minutes of inactivity, a node becomes questionable.
    Questionable,

    /// Nodes become bad when they fail to respond to multiple queries in a row. At this point, they
    /// are not sent to other peers. They are replaced with new good nodes.
    Bad,
}

impl Node {
    pub fn new(id: NodeID, address: SocketAddrV4, announce_token: Vec<u8>) -> Node {
        let epoch = NaiveDate::from_ymd(1970, 1, 1).and_hms_milli(0, 0, 1, 980);

        Node {
            id,
            address,
            announce_token,
            token: rand::random(),
            last_token: rand::random(),
            last_request_to: epoch,
            last_request_from: epoch,
            failed_requests: 0,
        }
    }

    /// Updates `last_token` and `token` moving `token` to `last_token` and creating a new `token`.
    /// Returns the new token.
    pub fn update_token(&mut self) -> u64 {
        self.last_token = self.token;
        self.token = rand::random();
        self.token
    }

    pub fn mark_successful_request(&mut self) {
        self.failed_requests = 0;
        self.last_request_to = Utc::now().naive_utc();
    }

    pub fn mark_failed_request(&mut self) {
        self.failed_requests += 1;
    }

    pub fn mark_successful_request_from(&mut self) {
        self.last_request_from = Utc::now().naive_utc();
    }

    pub fn state(&self) -> NodeState {
        let now = Utc::now().naive_utc();
        let since_last_request_to = now.signed_duration_since(self.last_request_to);
        let since_last_request_from = now.signed_duration_since(self.last_request_from);

        if self.failed_requests >= 2 {
            NodeState::Bad
        } else if since_last_request_to.num_minutes() < 15
            || since_last_request_from.num_minutes() < 15
        {
            NodeState::Good
        } else {
            NodeState::Questionable
        }
    }

    pub fn verify_token(&self, token: u64) -> bool {
        self.token == token || self.last_token == token
    }
}

enum FindNodeResult {
    Node(NodeInfo),
    Nodes(Vec<NodeInfo>),
}

const MAX_BUCKET_SIZE: usize = 8;

struct Bucket {
    /// Inclusive start key of nodes in the bucket.
    start: NodeID,

    /// Exclusive end key of nodes in the bucket.
    end: NodeID,

    /// Nodes in the bucket. These nodes could be in any state.
    nodes: Vec<Node>,
}

impl Bucket {
    pub fn new(start: NodeID, end: NodeID) -> Bucket {
        Bucket {
            start,
            end,
            nodes: Vec::new(),
        }
    }

    pub fn could_hold_node(&self, id: &NodeID) -> bool {
        id.deref() >= self.start.deref() && id.deref() < self.end.deref()
    }

    fn midpoint(&self) -> NodeID {
        NodeID::new(self.start.deref() + (self.end.deref() - self.start.deref()) / 2u8)
    }

    pub fn split(&mut self) -> Bucket {
        let midpoint = self.midpoint();

        let next_bucket_end = mem::replace(&mut self.end, midpoint.clone());
        let mut next_bucket = Bucket::new(midpoint, next_bucket_end);

        let previous_bucket_nodes = Vec::with_capacity(MAX_BUCKET_SIZE);
        let mut all_nodes = mem::replace(&mut self.nodes, previous_bucket_nodes);

        for node in all_nodes.drain(..) {
            let nodes = if self.could_hold_node(&node.id) {
                &mut self.nodes
            } else {
                &mut next_bucket.nodes
            };

            nodes.push(node);
        }

        next_bucket
    }

    pub fn is_full(&self) -> bool {
        self.good_nodes().count() >= MAX_BUCKET_SIZE
    }

    pub fn add_node(&mut self, node: Node) {
        if self.nodes.len() < MAX_BUCKET_SIZE {
            self.nodes.push(node);
            return;
        }

        let bad_node_opt = self
            .nodes
            .iter_mut()
            .find(|node| node.state() == NodeState::Bad);
        if let Some(bad_node) = bad_node_opt {
            mem::replace(bad_node, node);
        }

        // TODO: Ping Questionable Node
    }

    pub fn good_nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .filter(|node| node.state() == NodeState::Good)
    }

    pub fn get(&self, id: &NodeID) -> Option<&Node> {
        self.nodes.iter().find(|node| &node.id == id)
    }
}

pub struct RoutingTable {
    /// Node identifier of the node which the table is based around. There will be more buckets
    /// closer to this identifier.
    id: NodeID,

    /// Ordered list of buckets covering the key space. The first bucket starts at key 0 and the
    /// last bucket ends at key 2^160.
    buckets: Vec<Bucket>,
}

impl RoutingTable {
    /// Adds a node to the routing table.
    pub fn add_node(&mut self, node: Node) {
        let bucket_idx = self.get_bucket_idx(&node.id);

        let bucket_to_add_to_idx = if self.buckets[bucket_idx].is_full() {
            if !self.buckets[bucket_idx].could_hold_node(&node.id) {
                return;
            }

            let (prev_bucket_idx, next_bucket_idx) = self.split_bucket(bucket_idx);

            if self.buckets[prev_bucket_idx].could_hold_node(&node.id) {
                prev_bucket_idx
            } else {
                next_bucket_idx
            }
        } else {
            bucket_idx
        };

        &mut self.buckets[bucket_to_add_to_idx].add_node(node);
    }

    /// Finds the node with `id`, or about the `k` nearest good nodes to the `id` if the exact node
    /// couldn't be found. More or less than `k` nodes may be returned.
    fn find_node(&self, id: &NodeID) -> FindNodeResult {
        let bucket_idx = self.get_bucket_idx(id);
        let bucket = &self.buckets[bucket_idx];

        match bucket.get(id) {
            None => FindNodeResult::Nodes(bucket.good_nodes().map(|node| node.into()).collect()),
            Some(node) => FindNodeResult::Node((node as &Node).into()),
        }
    }

    /// Finds nodes in the same bucket as `id` in the routing table.
    fn find_nodes(&self, id: &NodeID) -> Vec<NodeInfo> {
        let bucket_idx = self.get_bucket_idx(id);
        let bucket = &self.buckets[bucket_idx];

        bucket.good_nodes().map(|node| node.into()).collect()
    }

    /// Gets the node with `id` from the table.
    fn get_node(&self, id: &NodeID) -> Option<&Node> {
        let bucket_idx = self.get_bucket_idx(id);
        let bucket = &self.buckets[bucket_idx];

        bucket.get(id)
    }

    /// Gets the index of the bucket which can hold `id`.
    fn get_bucket_idx(&self, id: &NodeID) -> usize {
        self.buckets
            .binary_search_by(|bucket| {
                if bucket.could_hold_node(id) {
                    cmp::Ordering::Equal
                } else {
                    bucket.start.cmp(id)
                }
            }).expect("No bucket was found for NodeID.")
    }

    /// Splits the bucket at `idx` into two buckets.
    fn split_bucket(&mut self, idx: usize) -> (usize, usize) {
        let next_bucket = {
            let mut bucket = &mut self.buckets[idx];
            bucket.split()
        };

        let next_bucket_idx = idx + 1;
        self.buckets.insert(next_bucket_idx, next_bucket);

        (idx, next_bucket_idx)
    }
}
