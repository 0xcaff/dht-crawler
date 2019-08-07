use crate::{
    contacts::Contacts,
    node_contact_state::{
        NodeContactState,
        NodeState,
    },
    transport::WrappedSendTransport,
};
use chrono::NaiveDateTime;
use futures::future::AbortHandle;
use krpc_encoding::NodeInfo;

pub enum Bucket {
    Leaf(LeafNode),
    Inner(Box<InnerNode>),
}

impl Bucket {
    fn split(&mut self) {
        unimplemented!()
    }

    fn add(&mut self, _node_info: NodeInfo) {
        unimplemented!()
    }
}

pub struct LeafNode {
    // TODO: Lower and Higher Bounds for Bucket Refresh

    contacts: Contacts,
    leaf_type: LeafType,

    /// Indicates how fresh the contents of the bucket is.
    ///
    /// Updated to the current time when:
    ///
    /// * a node in a bucket is pinged and it responds
    /// * a node is added to a bucket
    /// * a node in a bucket is replaced with another node
    last_updated: NaiveDateTime,

    /// Handle to cancel a bucket refresh.
    ///
    /// Buckets are refreshed 15 minutes after the last time [`last_updated`] is
    /// updated.
    periodic_update_handle: AbortHandle,
}

impl LeafNode {
    async fn try_add<'a>(
        &'a mut self,
        node_info: &NodeInfo,
        send_transport: &WrappedSendTransport,
    ) -> Option<&'a mut NodeContactState> {
        if let Some(idx) = self.contacts.update_node_id(&node_info.node_id) {
            return Some(self.contacts.get_mut(idx));
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

            send_transport.ping(&mut questionable_node).await;

            if let Some(idx) = self.contacts.update_node_id(&node_info.node_id) {
                return Some(self.contacts.get_mut(idx));
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

    // TODO: Periodic Table Update
}

// TODO: 0, 1 -> left, right

/// Non-leaf node in the K-Bucket tree.
pub struct InnerNode {
    pub left: Bucket,
    pub right: Bucket,
}

#[derive(PartialEq)]
enum LeafType {
    /// This bucket is near our node id. When it becomes too big, it is split.
    Near,

    /// This bucket is far from our node id. When it becomes too big, new nodes
    /// are ignored.
    Far,
}
