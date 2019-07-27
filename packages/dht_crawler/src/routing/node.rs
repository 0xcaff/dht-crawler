use chrono::{
    NaiveDateTime,
    Utc,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use std::net::SocketAddrV4;

#[derive(Debug, PartialEq)]
pub struct Node {
    pub id: NodeID,
    pub address: SocketAddrV4,

    /// Last time a message was sent from ourselves to this node and a response
    /// was received successfully.
    last_request_to: Option<NaiveDateTime>,

    /// Last time a valid request was received from this node.
    last_request_from: Option<NaiveDateTime>,

    /// Number of failed requests from us to the node since `last_request_to`.
    failed_requests: u8,
}

impl<'a> Into<NodeInfo> for &'a Node {
    fn into(self) -> NodeInfo {
        NodeInfo::new(self.id.clone(), self.address)
    }
}

impl Into<NodeInfo> for Node {
    fn into(self) -> NodeInfo {
        NodeInfo::new(self.id, self.address)
    }
}

#[derive(Debug, PartialEq)]
pub enum NodeState {
    /// A good node is a node has responded to one of our queries within the
    /// last 15 minutes. A node is also good if it has ever responded to one
    /// of our queries and has sent us a query within the last 15 minutes.
    Good,

    /// After 15 minutes of inactivity, a node becomes questionable.
    Questionable,

    /// Nodes become bad when they fail to respond to multiple queries in a row.
    /// At this point, they are not sent to other peers. They are replaced
    /// with new good nodes.
    Bad,
}

impl Node {
    pub fn new(id: NodeID, address: SocketAddrV4) -> Node {
        Node {
            id,
            address,
            last_request_to: None,
            last_request_from: None,
            failed_requests: 0,
        }
    }

    pub fn mark_successful_request(&mut self) {
        self.failed_requests = 0;
        self.last_request_to = Some(Utc::now().naive_utc());
    }

    pub fn mark_failed_request(&mut self) {
        self.failed_requests += 1;
    }

    pub fn mark_successful_request_from(&mut self) {
        self.last_request_from = Some(Utc::now().naive_utc());
    }

    pub fn state(&self) -> NodeState {
        let now = Utc::now().naive_utc();

        if self.failed_requests >= 2 {
            return NodeState::Bad;
        };

        match (self.last_request_from, self.last_request_to) {
            (Some(last_request_from), Some(..))
                if now.signed_duration_since(last_request_from).num_minutes() < 15 =>
            {
                NodeState::Good
            }
            (_, Some(last_request_to))
                if now.signed_duration_since(last_request_to).num_minutes() < 15 =>
            {
                NodeState::Good
            }
            _ => NodeState::Questionable,
        }
    }

    #[cfg(test)]
    pub fn new_with_id(id: u8) -> Node {
        use num_bigint::BigUint;

        let addr: SocketAddrV4 = "127.0.0.1:3000".parse().unwrap();

        Node::new(NodeID::new(BigUint::from(id)), addr.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Node,
        NodeID,
        NodeState,
    };
    use chrono::{
        prelude::*,
        Duration,
    };
    use failure::Error;
    use num_bigint::BigUint;

    #[test]
    fn starting_state() {
        let node = Node::new_with_id(10u8);

        assert_eq!(node.state(), NodeState::Questionable);
    }

    #[test]
    fn good_state_request() {
        let mut node = Node::new_with_id(10);
        node.mark_successful_request();

        assert_eq!(node.state(), NodeState::Good);
    }

    #[test]
    fn response_only_questionable() {
        let mut node = Node::new_with_id(10);
        node.mark_successful_request_from();

        assert_eq!(node.state(), NodeState::Questionable);
    }

    #[test]
    fn bad_state() {
        let mut node = Node::new_with_id(10);
        node.mark_failed_request();
        assert_eq!(node.state(), NodeState::Questionable);

        node.mark_failed_request();
        assert_eq!(node.state(), NodeState::Bad);
    }

    #[test]
    fn request_response_good() -> Result<(), Error> {
        let epoch = NaiveDate::from_ymd(1970, 1, 1).and_hms_milli(0, 0, 1, 980);

        let node = Node {
            id: NodeID::new(BigUint::from(10u8)),
            address: "127.0.0.1:3000".parse()?,
            last_request_to: Some(epoch),
            last_request_from: Some(Utc::now().naive_utc() - Duration::minutes(10)),
            failed_requests: 0,
        };

        assert_eq!(node.state(), NodeState::Good);

        Ok(())
    }
}
