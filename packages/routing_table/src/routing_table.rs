use crate::{
    full_b_tree::FullBTreeNode,
    k_bucket::KBucket,
    node_contact_state::NodeContactState,
    transport::WrappedSendTransport,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use tokio_krpc::{
    RequestTransport,
    SendTransport,
};

/// A routing table which holds information about nodes in the network.
pub struct RoutingTable {
    id: NodeID,
    root: FullBTreeNode<KBucket>,
    send_transport: WrappedSendTransport,
}

impl RoutingTable {
    pub fn new(id: NodeID, request_transport: RequestTransport) -> RoutingTable {
        RoutingTable {
            id,
            root: FullBTreeNode::Leaf(KBucket::initial()),
            send_transport: WrappedSendTransport::new(request_transport),
        }
    }

    /// Tries to add a node to the routing table.
    ///
    /// If a bucket is full:
    /// * and the bucket is a near bucket, it is split.
    /// * and is a far bucket or can no-longer be split, first bad nodes are
    ///   evicted, then and questionable nodes are queried. If any questionable
    ///   nodes turn out to be bad they are evicted.
    ///
    /// If there's no where to put a node, it is not added to the routing table.
    pub async fn add_node<'a>(
        &'a mut self,
        node_info: &NodeInfo,
    ) -> Option<&'a mut NodeContactState> {
        let (depth, bucket) = Self::find_bucket_from(&mut self.root, &node_info.node_id, 0);
        let leaf_bucket = bucket.unwrap_as_leaf();
        if let Some(idx) = leaf_bucket.try_add(node_info, &self.send_transport).await {
            unsafe {
                return Some((*(leaf_bucket as *mut KBucket)).get_mut(idx));
            }
        }

        if leaf_bucket.can_split() {
            bucket.split(&self.id, depth);

            let (_next_depth, next_bucket) =
                Self::find_bucket_from(bucket, &node_info.node_id, depth);
            let next_leaf_bucket = next_bucket.unwrap_as_leaf();
            if let Some(idx) = next_leaf_bucket
                .try_add(node_info, &self.send_transport)
                .await
            {
                unsafe {
                    return Some((*(next_leaf_bucket as *mut KBucket)).get_mut(idx));
                }
            }
        }

        None
    }

    pub fn find_node(&self, _id: &NodeID) -> FindNodeResult {
        unimplemented!()
    }

    fn find_bucket_from<'a>(
        root: &'a mut FullBTreeNode<KBucket>,
        node_id: &NodeID,
        mut depth: usize,
    ) -> (usize, &'a mut FullBTreeNode<KBucket>) {
        let mut b_tree_node = root;

        loop {
            match b_tree_node {
                FullBTreeNode::Leaf(_) => return (depth, b_tree_node),
                FullBTreeNode::Inner(inner) => {
                    let bit = node_id.nth_bit(depth);
                    depth += 1;

                    b_tree_node = if bit {
                        &mut inner.left
                    } else {
                        &mut inner.right
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
