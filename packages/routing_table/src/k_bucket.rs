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
use log::{
    as_error,
    debug,
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

    pub fn get_node_index(&self, node_id: &NodeID) -> Option<usize> {
        self.contacts
            .iter()
            .enumerate()
            .find(|(_, node)| &node.id == node_id)
            .map(|(idx, _node)| idx)
    }

    pub fn get_node_mut(&mut self, index: usize) -> &mut NodeContactState {
        &mut self.contacts[index]
    }

    pub fn good_nodes(&self) -> impl Iterator<Item = NodeInfo> + '_ {
        self.contacts
            .iter()
            .filter(|it| it.state() == NodeState::Good)
            .map(|it| NodeInfo::new(it.id.clone(), it.address.clone()))
    }

    /// Removes a bad node if there is one.
    pub fn take_bad_node(&mut self) -> Option<NodeContactState> {
        let idx = self
            .contacts
            .iter()
            .enumerate()
            .filter(|(_idx, it)| it.state() == NodeState::Bad)
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
            .filter(|(_idx, it)| it.state() == NodeState::Questionable)
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

    fn add_node(&mut self, node_info: &NodeInfo) -> usize {
        let node_contact_state =
            NodeContactState::new(node_info.node_id.clone(), node_info.address);

        self.contacts.push(node_contact_state);
        let len = self.contacts.len();
        len - 1
    }

    /// Try to add node to this bucket. If the bucket is full, first tries to
    /// evict bad nodes then tries to evict questionable nodes. If all fails,
    /// returns control to the caller to handle splitting the bucket.
    pub async fn try_add(
        &mut self,
        node_info: &NodeInfo,
        transport: &LivenessTransport,
    ) -> Option<usize> {
        // It is necessary to split this into a check and then a separate get
        // which does not borrow self because of limitations in the borrow
        // checker.
        // https://blog.rust-lang.org/2022/08/05/nll-by-default.html
        if let Some(node_index) = self.get_node_index(&node_info.node_id) {
            return Some(node_index);
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
            // try to evict questionable nodes until there are no more
            // questionable nodes
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

    pub fn split(&mut self, owner_id: &NodeID, depth: usize) -> (KBucket, KBucket) {
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

        let result = request_transport.ping(&mut questionable_node).await;

        match result {
            Ok(_) => {}
            Err(err) => {
                debug!(err = as_error!(err); "ping failed")
            }
        };

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
    use krpc_encoding::NodeID;
    type Error = Box<dyn std::error::Error>;

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

// todo: write tests (run coverage and see what's missing)
