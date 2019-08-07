use crate::bucket::{
    Bucket,
    LeafNode,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use tokio_krpc::SendTransport;

/// A routing table which holds information about nodes in the network.
pub struct RoutingTable {
    root: Bucket,
    send_transport: SendTransport,
}

impl RoutingTable {
    /// Tries to add a node to the routing table.
    ///
    /// If a bucket is full:
    /// * and the bucket is a near bucket, it is split.
    /// * and is a far bucket or can no-longer be split, first bad nodes are
    ///   evicted, then and questionable nodes are queried. If any questionable
    ///   nodes turn out to be bad they are evicted.
    ///
    /// If there's no where to put a node, it is not added to the routing table.
    pub async fn add_node(&mut self, _node_info: NodeInfo) {
        unimplemented!()
    }

    pub fn find_node(&self, _id: &NodeID) -> FindNodeResult {
        unimplemented!()
    }

    /// Finds the [`LeafNode`] which `node_id` will go into.
    fn find_bucket(&mut self, node_id: NodeID) -> &mut LeafNode {
        let mut bucket = &mut self.root;
        let mut bit_idx: usize = 0;

        loop {
            match bucket {
                Bucket::Leaf(leaf) => return leaf,
                Bucket::Inner(inner) => {
                    let bit = node_id.nth_bit(bit_idx);
                    bit_idx += 1;

                    if bit {
                        bucket = &mut inner.left
                    } else {
                        bucket = &mut inner.right
                    }
                }
            }
        }
    }
}

pub enum FindNodeResult {
    Node(NodeInfo),
    Nodes(Vec<NodeInfo>),
}
