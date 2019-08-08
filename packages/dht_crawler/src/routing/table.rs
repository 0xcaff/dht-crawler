use crate::routing::{
    bucket::Bucket,
    node::Node,
    token_validator::TokenValidator,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use std::{
    cmp,
    net::SocketAddrV4,
};

pub enum FindNodeResult {
    Node(NodeInfo),
    Nodes(Vec<NodeInfo>),
}

pub struct RoutingTable {
    /// Node identifier of the node which the table is based around. There will
    /// be more buckets closer to this identifier.
    id: NodeID,

    /// Ordered list of buckets covering the key space. The first bucket starts
    /// at key 0 and the last bucket ends at key 2^160.
    buckets: Vec<Bucket>,

    token_validator: TokenValidator,
}

impl RoutingTable {
    pub fn new(id: NodeID) -> RoutingTable {
        let mut buckets = Vec::new();
        buckets.push(Bucket::initial_bucket());

        RoutingTable {
            id,
            buckets,
            token_validator: TokenValidator::new(),
        }
    }

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

    /// Finds the node with `id`, or about the `k` nearest good nodes to the
    /// `id` if the exact node couldn't be found. More or less than `k`
    /// nodes may be returned.
    pub fn find_node(&self, id: &NodeID) -> FindNodeResult {
        let bucket_idx = self.get_bucket_idx(id);
        let bucket = &self.buckets[bucket_idx];

        match bucket.get(id) {
            None => FindNodeResult::Nodes(bucket.good_nodes().map(|node| node.into()).collect()),
            Some(node) => FindNodeResult::Node((node as &Node).into()),
        }
    }

    /// Finds nodes in the same bucket as `id` in the routing table.
    pub fn find_nodes(&self, id: &NodeID) -> Vec<NodeInfo> {
        let bucket_idx = self.get_bucket_idx(id);
        let bucket = &self.buckets[bucket_idx];

        bucket.good_nodes().map(|node| node.into()).collect()
    }

    /// Gets the node with `id` from the table.
    pub fn get_node(&self, id: &NodeID) -> Option<&Node> {
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
            })
            .expect("No bucket was found for NodeID.")
    }

    /// Splits the bucket at `idx` into two buckets.
    fn split_bucket(&mut self, idx: usize) -> (usize, usize) {
        let next_bucket = {
            let bucket = &mut self.buckets[idx];
            bucket.split()
        };

        let next_bucket_idx = idx + 1;
        self.buckets.insert(next_bucket_idx, next_bucket);

        (idx, next_bucket_idx)
    }

    pub fn verify_token(&self, token: &[u8], addr: &SocketAddrV4) -> bool {
        self.token_validator.verify_token(addr, token)
    }

    pub fn generate_token(&self, addr: &SocketAddrV4) -> [u8; 20] {
        self.token_validator.generate_token(addr)
    }

    pub fn update_token(&mut self) {
        self.token_validator.rotate_tokens();
    }

    pub fn get_or_add(&mut self, id: NodeID, address: SocketAddrV4) -> Option<&mut Node> {
        let bucket_idx = self.get_bucket_idx(&id);
        let bucket = &mut self.buckets[bucket_idx];

        if bucket.get(&id).is_none() {
            bucket.add_node(Node::new(id.clone(), address));
        }

        bucket.get_mut(&id)
    }

    pub fn len(&self) -> usize {
        self.buckets.iter().map(|bucket| bucket.nodes.len()).sum()
    }
}
