use crate::{
    full_b_tree::FullBTreeNode,
    generator::{
        GeneratorExt,
        GeneratorToIterator,
    },
    k_bucket::KBucket,
    node_contact_state::NodeContactState,
    transport::LivenessTransport,
};
use async_recursion::async_recursion;
use krpc_encoding::{
    NodeID,
    NodeInfo,
    NODE_ID_SIZE_BITS,
};
use std::ops::Generator;
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
        Self::add_node_rec(&self.id, &self.transport, &mut self.root, node_info, 0).await
    }

    fn find_nodes_generator_rec(
        root: &FullBTreeNode<KBucket>,
        node_id: NodeID,
        depth: usize,
    ) -> Box<dyn Iterator<Item = NodeInfo> + '_> {
        Box::new(
            (move || match root {
                FullBTreeNode::Inner(ref inner) => {
                    let bit = node_id.nth_bit(depth);
                    let (matching_branch, other_branch) = if bit {
                        (&inner.left, &inner.right)
                    } else {
                        (&inner.right, &inner.left)
                    };

                    for value in
                        Self::find_nodes_generator_rec(matching_branch, node_id.clone(), depth + 1)
                    {
                        yield value;
                    }

                    for value in
                        Self::find_nodes_generator_rec(other_branch, node_id.clone(), depth + 1)
                    {
                        yield value;
                    }
                }
                FullBTreeNode::Leaf(values) => {
                    for node in values.good_nodes() {
                        yield node;
                    }
                }
            })
            .iter(),
        )
    }

    fn find_nodes_generator(&self) -> impl Iterator<Item = NodeInfo> + '_ {
        Self::find_nodes_generator_rec(&self.root, self.id.clone(), 0)
    }

    pub fn find_node(&self, id: &NodeID) -> FindNodeResult {
        let closest_nodes = self
            .find_nodes_generator()
            .into_iter()
            .take(8)
            .collect::<Vec<NodeInfo>>();

        match closest_nodes
            .iter()
            .enumerate()
            .find(|(_, node)| &node.node_id == id)
            .map(|(idx, _)| idx)
        {
            Some(node) => FindNodeResult::Node(closest_nodes[node].clone()),
            None => FindNodeResult::Nodes(closest_nodes),
        }
    }

    fn find_bucket_mut_recursive<'a>(
        root: &'a mut FullBTreeNode<KBucket>,
        node_id: &NodeID,
        depth: usize,
    ) -> (&'a mut FullBTreeNode<KBucket>, usize) {
        match root {
            FullBTreeNode::Inner(ref mut inner) => {
                let bit = node_id.nth_bit(depth);
                let root = if bit {
                    &mut inner.left
                } else {
                    &mut inner.right
                };

                return Self::find_bucket_mut_recursive(root, node_id, depth + 1);
            }
            FullBTreeNode::Leaf(_) => (root, depth),
        }
    }

    #[async_recursion(?Send)]
    async fn add_node_rec<'a>(
        owner_id: &NodeID,
        transport: &LivenessTransport,
        root_node: &'a mut FullBTreeNode<KBucket>,
        node_info: &NodeInfo,
        starting_depth: usize,
    ) -> Option<&'a mut NodeContactState> {
        let (leaf_bucket, depth) =
            Self::find_bucket_mut_recursive(root_node, &node_info.node_id, starting_depth);

        let leaf_k_bucket = leaf_bucket.unwrap_as_leaf();

        let result = leaf_k_bucket.try_add(node_info, transport).await;

        if let Some(node_index) = result {
            let mut raw = leaf_k_bucket as *mut KBucket;

            // Ignore the borrow checker, it is incorrect here. We can safely
            // return a reference here because the branches of the if statement
            // are exclusive. If this if statement executes, we can safely take
            // a mutable borrow over leaf_k_bucket. If it does not, the split
            // can safely take a reference over leaf_bucket.
            // https://blog.rust-lang.org/2022/08/05/nll-by-default.html
            unsafe {
                let raw_ref = &mut *raw;
                return Some(raw_ref.get_node_mut(node_index));
            }
        }

        if !leaf_k_bucket.can_split() {
            return None;
        }

        // don't allow a tree with more than 160 levels
        if depth >= NODE_ID_SIZE_BITS - 1 {
            return None;
        }

        leaf_bucket.split(owner_id, depth);

        Self::add_node_rec(owner_id, transport, leaf_bucket, node_info, depth).await
    }
}

pub enum FindNodeResult {
    Node(NodeInfo),
    Nodes(Vec<NodeInfo>),
}
