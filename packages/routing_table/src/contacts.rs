use crate::node_contact_state::{
    NodeContactState,
    NodeState,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use std::cmp::Ordering;

const K_BUCKET_SIZE: usize = 8;

/// A container of recently contacted nodes.
pub struct Contacts {
    contacts: Vec<NodeContactState>,
}

impl Contacts {
    pub fn new() -> Contacts {
        Contacts {
            contacts: Vec::new(),
        }
    }

    fn from_existing(contacts: Vec<NodeContactState>) -> Contacts {
        Contacts { contacts }
    }

    pub fn get_node_mut(&mut self, node_id: &NodeID) -> Option<&mut NodeContactState> {
        self.contacts
            .iter_mut()
            .find(|node| &node.id == node_id)
    }

    fn get_least_recently_contacted(&self, state: NodeState) -> Option<usize> {
        self.contacts
            .iter()
            .enumerate()
            .filter(|(idx, it)| it.state() == state)
            .min_by(
                |(_, lhs), (_, rhs)| match (lhs.last_contacted(), rhs.last_contacted()) {
                    (None, None) => Ordering::Equal,
                    (Some(lhs_last_contacted), Some(rhs_last_contacted)) => {
                        lhs_last_contacted.cmp(&rhs_last_contacted)
                    }
                    (Some(_), None) => Ordering::Greater,
                    (None, Some(_)) => Ordering::Less,
                },
            )
            .map(|(idx, _ )| idx)
    }

    /// Removes the least recently seen bad node if there is one.
    pub fn take_bad_node(&mut self) -> Option<NodeContactState> {
        self.get_least_recently_contacted(NodeState::Bad)
            .map(|idx| self.contacts.remove(idx))
    }

    /// Returns the least recently seen questionable node.
    pub fn take_questionable_node(&mut self) -> Option<NodeContactState> {
        self.get_least_recently_contacted(NodeState::Questionable)
            .map(|idx| self.contacts.remove(idx))
    }

    /// Returns true if there definitely is space in the bucket.
    pub fn definitely_has_remaining_space(&self) -> bool {
        self.contacts.len() < K_BUCKET_SIZE
    }

    pub fn get_mut(&mut self, index: usize) -> &mut NodeContactState {
        &mut self.contacts[index]
    }

    pub fn add_new(&mut self, node_info: &NodeInfo) -> usize {
        // todo: initialization condition
        let node_contact_state =
            NodeContactState::new(node_info.node_id.clone(), node_info.address);

        self.contacts.push(node_contact_state);

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

#[cfg(test)]
mod tests {
    use crate::{
        contacts::Contacts,
        node_contact_state::NodeContactState,
    };
    use krpc_encoding::{
        NodeID,
    };
    use failure::Error;

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

        let mut contacts = Contacts::from_existing(vec![questionable_node, bad_node]);

        assert_eq!(
            contacts.take_bad_node().map(|node| node.id),
            Some(bad_node_id)
        );

        Ok(())
    }
}
