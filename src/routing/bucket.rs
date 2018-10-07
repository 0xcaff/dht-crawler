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

#[cfg(test)]
mod tests {
    use super::{BigUint, Bucket, NodeID};
    use num;

    #[test]
    fn lower_bound_initial_bucket() {
        let bucket = Bucket::initial_bucket();
        let lower_bound = BigUint::from(0u8);

        assert!(bucket.could_hold_node(&NodeID::new(lower_bound)));
    }

    #[test]
    fn upper_bound_initial_bucket() {
        let bucket = Bucket::initial_bucket();
        let upper_bound = BigUint::from_bytes_be(&[0xffu8; 20]);

        assert!(bucket.could_hold_node(&NodeID::new(upper_bound)));
    }

    #[test]
    fn inner_value_initial_bucket() {
        let bucket = Bucket::initial_bucket();
        let value = BigUint::from(80192381092u128);

        assert!(bucket.could_hold_node(&NodeID::new(value)));
    }

    #[test]
    fn outside_upper_bound_initial_bucket() {
        let bucket = Bucket::initial_bucket();
        let value = BigUint::from_bytes_be(&[0xffu8; 20]) + 10u8;

        assert!(!bucket.could_hold_node(&NodeID::new(value)));
    }

    #[test]
    fn initial_bucket_midpoint() {
        let bucket = Bucket::initial_bucket();
        let expected_midpoint = num::pow(BigUint::from(2u8), 159);

        assert_eq!(expected_midpoint, *bucket.midpoint());
    }

    #[test]
    fn after_beginning_midpoint() {
        let start = NodeID::new(BigUint::from(10u8));
        let end = NodeID::new(BigUint::from(20u8));
        let bucket = Bucket::new(start, end);
        assert_eq!(BigUint::from(15u8), *bucket.midpoint());
    }
}
