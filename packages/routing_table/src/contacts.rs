use crate::node_contact_state::{
    NodeContactState,
    NodeState,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};

const K_BUCKET_SIZE: usize = 8;

/// A container of recently contacted nodes sorted from most recently accessed
/// to least recently accessed.
pub struct Contacts {
    // todo: say something about how this is sorted
    contacts: Vec<NodeContactState>,

    // todo: maybe sort at read time instead of write time
}

impl Contacts {
    pub fn new() -> Contacts {
        Contacts {
            contacts: Vec::new(),
        }
    }

    pub(crate) fn from_existing(contacts: Vec<NodeContactState>) -> Contacts {
        Contacts { contacts }
    }

    /// Gets a node by id. Returns [`None`] if the node_id isn't found.
    pub fn update_node_id(&mut self, node_id: &NodeID) -> Option<usize> {
        let index = self
            .contacts
            .iter()
            .enumerate()
            .find(|(_index, node)| &node.id == node_id)
            .map(|(index, _node)| index)?;

        let node = self.contacts.remove(index);
        self.contacts.insert(0, node);

        Some(0)
    }

    /// Tries to removes the least recently seen bad node. Returns whether or
    /// not a node was removed.
    pub fn try_remove_bad_node(&mut self) -> bool {
        let bad_node_index = self
            .contacts
            .iter()
            .rev()
            .enumerate()
            .find(|(_index, node)| node.state() == NodeState::Bad)
            .map(|(index, _node)| index);

        bad_node_index
            .map(|bad_node_index| self.contacts.remove(bad_node_index))
            .is_some()
    }

    /// Returns the least recently seen questionable node.
    pub fn questionable_node(&mut self) -> Option<(usize, NodeContactState)> {
        let (index, _) = self
            .contacts
            .iter()
            .rev()
            .enumerate()
            .find(|(_index, node)| node.state() == NodeState::Questionable)?;

        let node = self.contacts.remove(index);

        Some((index, node))
    }

    /// Returns true if there definitely is space in the bucket.
    pub fn has_remaining_space(&self) -> bool {
        self.contacts.len() < K_BUCKET_SIZE
    }

    pub fn get_mut(&mut self, index: usize) -> &mut NodeContactState {
        &mut self.contacts[index]
    }

    pub fn insert_back(&mut self, index: usize, node: NodeContactState) {
        self.contacts.insert(index, node)
    }

    pub fn insert_front(&mut self, node: NodeContactState) {
        self.contacts.insert(0, node)
    }

    pub fn add_new(&mut self, node_info: &NodeInfo) -> usize {
        let node_contact_state =
            NodeContactState::new(node_info.node_id.clone(), node_info.address);

        self.insert_front(node_contact_state);

        0
    }

    pub fn split(&mut self, depth: usize) -> (Contacts, Contacts) {
        let (zero_bit_nodes, one_bit_nodes) = self
            .contacts
            .drain(..)
            .partition(|node| !node.id.nth_bit(depth));

        (
            Contacts::from_existing(zero_bit_nodes),
            Contacts::from_existing(one_bit_nodes),
        )
    }
}
