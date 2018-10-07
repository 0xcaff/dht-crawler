use std::net::SocketAddrV4;

use chrono::{NaiveDate, NaiveDateTime, Utc};
use proto::{NodeID, NodeInfo};
use rand;

pub struct Node {
    pub id: NodeID,
    pub address: SocketAddrV4,

    /// Token received when discovering this node. Used when announcing ourselves to this peer.
    announce_token: Vec<u8>,

    /// Token which should be presented to us when announcing a peer. Updated every 10 minutes.
    token: u64,

    /// Last valid token which could also be presented when a peer announces. Updated every 10
    /// minutes.
    last_token: u64,

    /// Last time a message was sent from ourselves to this node and a response was received
    /// successfully.
    last_request_to: NaiveDateTime,

    /// Last time a valid request was received from this node.
    last_request_from: NaiveDateTime,

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

#[derive(PartialEq)]
pub enum NodeState {
    /// A good node is a node has responded to one of our queries within the last 15 minutes. A node
    /// is also good if it has ever responded to one of our queries and has sent us a query within
    /// the last 15 minutes.
    Good,

    /// After 15 minutes of inactivity, a node becomes questionable.
    Questionable,

    /// Nodes become bad when they fail to respond to multiple queries in a row. At this point, they
    /// are not sent to other peers. They are replaced with new good nodes.
    Bad,
}

impl Node {
    pub fn new(id: NodeID, address: SocketAddrV4, announce_token: Vec<u8>) -> Node {
        let epoch = NaiveDate::from_ymd(1970, 1, 1).and_hms_milli(0, 0, 1, 980);

        Node {
            id,
            address,
            announce_token,
            token: rand::random(),
            last_token: rand::random(),
            last_request_to: epoch,
            last_request_from: epoch,
            failed_requests: 0,
        }
    }

    /// Updates `last_token` and `token` moving `token` to `last_token` and creating a new `token`.
    /// Returns the new token.
    pub fn update_token(&mut self) -> u64 {
        self.last_token = self.token;
        self.token = rand::random();
        self.token
    }

    pub fn mark_successful_request(&mut self) {
        self.failed_requests = 0;
        self.last_request_to = Utc::now().naive_utc();
    }

    pub fn mark_failed_request(&mut self) {
        self.failed_requests += 1;
    }

    pub fn mark_successful_request_from(&mut self) {
        self.last_request_from = Utc::now().naive_utc();
    }

    pub fn state(&self) -> NodeState {
        let now = Utc::now().naive_utc();
        let since_last_request_to = now.signed_duration_since(self.last_request_to);
        let since_last_request_from = now.signed_duration_since(self.last_request_from);

        if self.failed_requests >= 2 {
            NodeState::Bad
        } else if since_last_request_to.num_minutes() < 15
            || since_last_request_from.num_minutes() < 15
        {
            NodeState::Good
        } else {
            NodeState::Questionable
        }
    }

    pub fn verify_token(&self, token: u64) -> bool {
        self.token == token || self.last_token == token
    }
}
