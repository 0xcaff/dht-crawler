use crate::{
    contacts::Contacts,
    full_b_tree::{
        FullBTreeInnerNode,
        FullBTreeNode,
    },
    node_contact_state::{
        NodeContactState,
        NodeState,
    },
    transport::WrappedSendTransport,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};

/// A bucket which holds a maximum of `k` nodes.
pub struct KBucket {
    contacts: Contacts,
    leaf_type: LeafType,
}

#[derive(PartialEq)]
enum LeafType {
    /// This bucket is near our node id. When it becomes too big, it is split.
    Near,

    /// This bucket is far from our node id. When it becomes too big, new nodes
    /// are ignored.
    Far,
}

impl KBucket {
    pub fn initial() -> KBucket {
        KBucket {
            contacts: Contacts::new(),
            leaf_type: LeafType::Near,
        }
    }

    pub async fn try_add<'a>(
        &mut self,
        node_info: &NodeInfo,
        send_transport: &WrappedSendTransport,
    ) -> Option<usize> {
        if let Some(idx) = self.contacts.update_node_id(&node_info.node_id) {
            return Some(idx);
        }

        // space in bucket
        if self.contacts.has_remaining_space() {
            return Some(self.contacts.add_new(node_info));
        }

        if self.contacts.try_remove_bad_node() {
            return Some(self.contacts.add_new(node_info));
        }

        loop {
            let questionable_node_opt = self.contacts.questionable_node();
            let (index, mut questionable_node) = match questionable_node_opt {
                Some(questionable_node) => questionable_node,
                None => return None,
            };

            let _ = send_transport.ping(&mut questionable_node).await;

            if let Some(idx) = self.contacts.update_node_id(&node_info.node_id) {
                return Some(idx);
            }

            match questionable_node.state() {
                NodeState::Bad => {
                    self.contacts.insert_back(index, questionable_node);
                    break;
                }
                NodeState::Questionable => {
                    self.contacts.insert_back(index, questionable_node);
                }
                NodeState::Good => {
                    self.contacts.insert_front(questionable_node);
                }
            }
        }

        if self.contacts.try_remove_bad_node() {
            return Some(self.contacts.add_new(node_info));
        }

        None
    }

    pub fn split(&mut self, right_is_near: bool, depth: usize) -> (KBucket, KBucket) {
        let (lhs_contacts, rhs_contacts) = self.contacts.split(depth);

        (
            KBucket {
                contacts: lhs_contacts,
                leaf_type: if right_is_near {
                    LeafType::Far
                } else {
                    LeafType::Near
                },
            },
            KBucket {
                contacts: rhs_contacts,
                leaf_type: if right_is_near {
                    LeafType::Near
                } else {
                    LeafType::Far
                },
            },
        )
    }

    pub fn get_mut(&mut self, idx: usize) -> &mut NodeContactState {
        self.contacts.get_mut(idx)
    }

    pub fn can_split(&self) -> bool {
        match self.leaf_type {
            LeafType::Far => false,
            LeafType::Near => true,
        }
    }

    // TODO: Periodic Bucket Refresh
}

impl FullBTreeNode<KBucket> {
    pub fn unwrap_as_leaf(&mut self) -> &mut KBucket {
        match self {
            FullBTreeNode::Leaf(leaf) => leaf,
            FullBTreeNode::Inner(_) => panic!("unwrap_as_leaf called on non-leaf"),
        }
    }

    pub fn split(&mut self, id: &NodeID, depth: usize) {
        let leaf = self.unwrap_as_leaf();
        let right_is_near = id.nth_bit(depth);
        let (lhs, rhs) = leaf.split(right_is_near, depth);

        *self = FullBTreeNode::Inner(Box::new(FullBTreeInnerNode {
            left: FullBTreeNode::Leaf(lhs),
            right: FullBTreeNode::Leaf(rhs),
        }));
    }
}
