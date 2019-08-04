use chrono::{
    NaiveDateTime,
    Utc,
};
use krpc_encoding::NodeID;
use std::net::SocketAddrV4;

pub struct NodeContactState {
    pub id: NodeID,

    pub address: SocketAddrV4,

    /// Last time a successful query was made to this node.
    last_successful_query_to: Option<NaiveDateTime>,

    /// Last time a valid query was received from this node.
    last_request_from: Option<NaiveDateTime>,

    /// Number of failed queries to the node since [`last_successful_query_to`].
    failed_queries: u8,
}

impl NodeContactState {
    pub fn new(id: NodeID, address: SocketAddrV4) -> Self {
        NodeContactState {
            id,
            address,
            last_successful_query_to: None,
            last_request_from: None,
            failed_queries: 0,
        }
    }

    /// Update internal state to reflect a successful query happened.
    pub fn mark_successful_query(&mut self) {
        self.failed_queries = 0;
        self.last_successful_query_to = Some(Utc::now().naive_utc());
    }

    /// Update internal state to reflect a query to this node failed.
    pub fn mark_failed_query(&mut self) {
        self.failed_queries += 1;
    }

    /// Update internal state to reflect a successful request has been received
    /// from a node.
    pub fn mark_successful_request(&mut self) {
        self.last_request_from = Some(Utc::now().naive_utc());
    }

    pub fn state(&self) -> NodeState {
        let now = Utc::now().naive_utc();

        if self.failed_queries >= 2 {
            return NodeState::Bad;
        };

        match (self.last_request_from, self.last_successful_query_to) {
            (_, Some(last_request_to))
                if now.signed_duration_since(last_request_to).num_minutes() < 15 =>
            {
                NodeState::Good
            }
            (Some(last_request_from), Some(..))
                if now.signed_duration_since(last_request_from).num_minutes() < 15 =>
            {
                NodeState::Good
            }
            _ => NodeState::Questionable,
        }
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

#[cfg(test)]
mod tests {
    use super::{
        NodeContactState,
        NodeState,
    };
    use chrono::{
        prelude::*,
        Duration,
    };
    use failure::Error;

    fn make_node() -> Result<NodeContactState, Error> {
        Ok(NodeContactState::new(
            b"0000000000000000000000000000000000000000".into(),
            "127.0.0.1:3000".parse()?,
        ))
    }

    #[test]
    fn starting_state() -> Result<(), Error> {
        let node = make_node()?;

        assert_eq!(node.state(), NodeState::Questionable);

        Ok(())
    }

    #[test]
    fn good_state_request() -> Result<(), Error> {
        let mut node = make_node()?;
        node.mark_successful_query();

        assert_eq!(node.state(), NodeState::Good);

        Ok(())
    }

    #[test]
    fn response_only_questionable() -> Result<(), Error> {
        let mut node = make_node()?;
        node.mark_successful_request();

        assert_eq!(node.state(), NodeState::Questionable);

        Ok(())
    }

    #[test]
    fn bad_state() -> Result<(), Error> {
        let mut node = make_node()?;
        node.mark_failed_query();
        assert_eq!(node.state(), NodeState::Questionable);

        node.mark_failed_query();
        assert_eq!(node.state(), NodeState::Bad);

        Ok(())
    }

    #[test]
    fn request_response_good() -> Result<(), Error> {
        let epoch = NaiveDate::from_ymd(1970, 1, 1).and_hms_milli(0, 0, 1, 980);

        let node = NodeContactState {
            id: b"0000000000000000000000000000000000000000".into(),
            address: "127.0.0.1:3000".parse()?,
            last_successful_query_to: Some(epoch),
            last_request_from: Some(Utc::now().naive_utc() - Duration::minutes(10)),
            failed_queries: 0,
        };

        assert_eq!(node.state(), NodeState::Good);

        Ok(())
    }
}
