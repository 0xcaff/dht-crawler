use std::mem;
use std::ops::Deref;

use proto::NodeID;

use bigint::BigUint;
use routing::node::{Node, NodeState};

const MAX_BUCKET_SIZE: usize = 8;

pub struct Bucket {
    /// Inclusive start key of nodes in the bucket.
    pub start: NodeID,

    /// Exclusive end key of nodes in the bucket.
    pub end: NodeID,

    /// Nodes in the bucket. These nodes could be in any state.
    pub nodes: Vec<Node>,
}

impl Bucket {
    pub fn new(start: NodeID, end: NodeID) -> Bucket {
        Bucket {
            start,
            end,
            nodes: Vec::new(),
        }
    }

    /// Creates a bucket spanning from key zero to key 2^160.
    pub fn initial_bucket() -> Bucket {
        let start = NodeID::new(BigUint::new(Vec::new()));
        let end = NodeID::new(BigUint::from_bytes_be(&[0xffu8; 20]) + 1u8);

        Bucket::new(start, end)
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
