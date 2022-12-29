use crate::{
    full_b_tree::FullBTreeNode,
    k_bucket::KBucket,
    node_contact_state::NodeContactState,
    transport::LivenessTransport,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
    NODE_ID_SIZE_BITS,
};
use tokio_krpc::RequestTransport;

/// A routing table which holds information about nodes in the network.
pub struct RoutingTable {
    id: NodeID,
    root: FullBTreeNode<KBucket>,
    transport: LivenessTransport,
}

impl RoutingTable {
    pub fn new(id: NodeID, request_transport: RequestTransport) -> RoutingTable {
        RoutingTable {
            id,
            root: FullBTreeNode::Leaf(KBucket::initial()),
            transport: LivenessTransport::new(request_transport),
        }
    }

    /// Tries to add a node to the routing table, evicting nodes which have
    /// gone offline and growing the routing table as needed.
    ///
    /// If the routing table is full, returns None.
    pub async fn add_node(&mut self, node_info: &NodeInfo) -> Option<&mut NodeContactState> {
        self.add_node_rec(&mut self.root, node_info, 0).await
    }

    pub fn find_node(&self, _id: &NodeID) -> FindNodeResult {
        // todo: implement
        unimplemented!()
    }

    fn find_bucket_mut_recursive<'a>(
        root: &'a mut FullBTreeNode<KBucket>,
        node_id: &NodeID,
        depth: usize,
    ) -> (&'a mut FullBTreeNode<KBucket>, usize) {
        match root {
            FullBTreeNode::Inner(mut inner) => {
                let bit = node_id.nth_bit(depth);

                return Self::find_bucket_mut_recursive(
                    &mut (if bit { inner.left } else { inner.right }),
                    node_id,
                    depth + 1,
                );
            }
            FullBTreeNode::Leaf(_) => (root, depth),
        }
    }

    async fn add_node_rec<'a>(
        &'a mut self,
        root_node: &'a mut FullBTreeNode<KBucket>,
        node_info: &NodeInfo,
        starting_depth: usize,
    ) -> Option<&'a mut NodeContactState> {
        let (leaf_bucket, depth) =
            Self::find_bucket_mut_recursive(root_node, &node_info.node_id, starting_depth);

        let leaf_k_bucket = leaf_bucket.unwrap_as_leaf();
        if let Some(node) = leaf_k_bucket.try_add(node_info, &self.transport).await {
            return Some(node);
        }

        if !leaf_k_bucket.can_split() {
            return None;
        }

        // don't allow a tree with more than 160 levels
        if depth >= NODE_ID_SIZE_BITS - 1 {
            return None;
        }

        leaf_bucket.split(&self.id, depth);

        self.add_node_rec(leaf_bucket, node_info, depth).await
    }
}

pub enum FindNodeResult {
    Node(NodeInfo),
    Nodes(Vec<NodeInfo>),
}
