use crate::{
    full_b_tree::{
        FullBTreeInnerNode,
        FullBTreeNode,
    },
    node_contact_state::{
        NodeContactState,
        NodeState,
    },
    transport::LivenessTransport,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use std::cmp::Ordering;

const K_BUCKET_SIZE: usize = 8;

/// A bucket which holds a maximum of `k` nodes.
pub struct KBucket {
    contacts: Vec<NodeContactState>,
    leaf_type: LeafType,
}

impl KBucket {
    pub fn initial() -> KBucket {
        KBucket {
            contacts: Vec::new(),
            leaf_type: LeafType::Near,
        }
    }

    pub fn get_node_mut(&mut self, node_id: &NodeID) -> Option<&mut NodeContactState> {
        self.contacts.iter_mut().find(|node| &node.id == node_id)
    }

    pub fn get_node(&self, node_id: &NodeID) -> Option<&NodeContactState> {
        self.contacts.iter().find(|node| &node.id == node_id)
    }

    /// Removes a bad node if there is one.
    pub fn take_bad_node(&mut self) -> Option<NodeContactState> {
        let idx = self
            .contacts
            .iter()
            .enumerate()
            .filter(|(idx, it)| it.state() == NodeState::Bad)
            .map(|(idx, _)| idx)
            .next()?;

        Some(self.contacts.remove(idx))
    }

    /// Returns the least recently seen questionable node.
    pub fn take_questionable_node(&mut self) -> Option<NodeContactState> {
        let idx = self
            .contacts
            .iter()
            .enumerate()
            .filter(|(idx, it)| it.state() == NodeState::Questionable)
            .min_by(|(_, lhs), (_, rhs)| {
                let failed_queries_cmp = lhs.failed_queries().cmp(&rhs.failed_queries());
                if let Ordering::Greater | Ordering::Less = failed_queries_cmp {
                    return failed_queries_cmp;
                }

                match (lhs.last_contacted(), rhs.last_contacted()) {
                    (None, None) => Ordering::Equal,
                    (Some(lhs_last_contacted), Some(rhs_last_contacted)) => {
                        lhs_last_contacted.cmp(&rhs_last_contacted)
                    }
                    (Some(_), None) => Ordering::Greater,
                    (None, Some(_)) => Ordering::Less,
                }
            })
            .map(|(idx, _)| idx)?;

        Some(self.contacts.remove(idx))
    }

    /// Returns true if there definitely is space in the bucket.
    pub fn definitely_has_remaining_space(&self) -> bool {
        self.contacts.len() < K_BUCKET_SIZE
    }

    fn add_node(&mut self, node_info: &NodeInfo) -> &mut NodeContactState {
        let node_contact_state =
            NodeContactState::new(node_info.node_id.clone(), node_info.address);

        self.contacts.push(node_contact_state);
        &mut self.contacts[self.contacts.len() - 1]
    }

    /// Try to add node to this bucket. If the bucket is full, first tries to
    /// evict bad nodes then tries to evict questionable nodes. If all fails,
    /// returns control to the caller to handle splitting the bucket.
    pub async fn try_add(
        &mut self,
        node_info: &NodeInfo,
        transport: &LivenessTransport,
    ) -> Option<&mut NodeContactState> {
        // node already exists, do not add it again
        {
            let self_ref = self;
            let retried_node =  self_ref.get_node_mut(&node_info.node_id);
            if let Some(node) = retried_node {
                return Some(node);
            }
        }

        // if there's space, add without worrying about evictions
        let has_space = self.definitely_has_remaining_space();
        if has_space {
            return Some(self.add_node(node_info));
        }

        // evict a bad node to make space
        if let Some(_) = self.take_bad_node() {
            return Some(self.add_node(node_info));
        }

        loop {
            // try to evict questionable nodes until there are no more questionable nodes
            match self.evict_questionable_node(transport).await {
                None => {
                    break;
                }
                Some(true) => {
                    return Some(self.add_node(node_info));
                }
                Some(false) => {
                    continue;
                }
            }
        }

        None
    }

    pub fn split(mut self, owner_id: &NodeID, depth: usize) -> (KBucket, KBucket) {
        let (zero_bit_nodes, one_bit_nodes) = self
            .contacts
            .drain(..)
            .partition(|node| !node.id.nth_bit(depth));
        let owner_is_one_bit = owner_id.nth_bit(depth);

        (
            KBucket {
                contacts: zero_bit_nodes,
                leaf_type: if owner_is_one_bit {
                    LeafType::Far
                } else {
                    LeafType::Near
                },
            },
            KBucket {
                contacts: one_bit_nodes,
                leaf_type: if owner_is_one_bit {
                    LeafType::Near
                } else {
                    LeafType::Far
                },
            },
        )
    }

    pub fn can_split(&self) -> bool {
        self.leaf_type.can_split()
    }

    /// Tries to evict a questionable node.
    ///
    /// Returns:
    /// * `None` if there are no questionable nodes to try and evict.
    /// * `Some(true)` if a node was evicted.
    /// * `Some(false)` if pinging the node resulted in the node going from
    ///   questionable to good
    pub async fn evict_questionable_node(
        &mut self,
        request_transport: &LivenessTransport,
    ) -> Option<bool> {
        let mut questionable_node = self.take_questionable_node()?;

        // todo: report the error somewhere

        let _ = request_transport.ping(&mut questionable_node).await;

        match questionable_node.state() {
            NodeState::Questionable | NodeState::Good => {
                self.contacts.push(questionable_node);
                Some(false)
            }
            NodeState::Bad => Some(true),
        }
    }
}

impl FullBTreeNode<KBucket> {
    pub fn unwrap_as_leaf(&mut self) -> &mut KBucket {
        match self {
            FullBTreeNode::Leaf(leaf) => leaf,
            FullBTreeNode::Inner(_) => panic!("unwrap_as_leaf called on non-leaf"),
        }
    }

    pub fn split(&mut self, owner_id: &NodeID, depth: usize) {
        let leaf = self.unwrap_as_leaf();
        let (lhs, rhs) = leaf.split(owner_id, depth);

        *self = FullBTreeNode::Inner(Box::new(FullBTreeInnerNode {
            left: FullBTreeNode::Leaf(lhs),
            right: FullBTreeNode::Leaf(rhs),
        }));
    }
}

#[derive(PartialEq)]
enum LeafType {
    /// This bucket is near our node id. When it becomes too big, it is split.
    Near,

    /// This bucket is far from our node id. When it becomes too big, new nodes
    /// are ignored.
    Far,
}

impl LeafType {
    fn can_split(&self) -> bool {
        match self {
            LeafType::Far => false,
            LeafType::Near => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        k_bucket::{
            KBucket,
            LeafType,
        },
        node_contact_state::NodeContactState,
    };
    use failure::Error;
    use krpc_encoding::NodeID;

    fn make_node() -> Result<NodeContactState, Error> {
        Ok(NodeContactState::new(
            NodeID::random(),
            "127.0.0.1:3000".parse()?,
        ))
    }

    #[test]
    fn test_take_bad_node() -> Result<(), Error> {
        let questionable_node = make_node()?;

        let mut bad_node = make_node()?;
        bad_node.mark_failed_query();
        bad_node.mark_failed_query();

        let bad_node_id = bad_node.id.clone();

        let mut contacts = KBucket {
            contacts: vec![questionable_node, bad_node],
            leaf_type: LeafType::Near,
        };

        assert_eq!(
            contacts.take_bad_node().map(|node| node.id),
            Some(bad_node_id)
        );

        Ok(())
    }
}
