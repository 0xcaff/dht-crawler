#![feature(async_await)]

mod node_contact_state;
mod routed_send_transport;

use crate::node_contact_state::{
    NodeContactState,
    NodeState,
};

use chrono::{
    NaiveDateTime,
    Utc,
};
use failure::Error;
use futures::{
    future::{
        self,
        AbortHandle,
    },
    Future,
};
use krpc_encoding::{
    NodeID,
    NodeInfo,
};
use std::{
    mem,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::prelude::FutureExt;
use tokio_krpc::SendTransport;

/// A routing table which holds information about nodes in the network.
struct RoutingTable {
    root: Bucket,
}

impl RoutingTable {
    /// Tries to add a node to the routing table.
    ///
    /// If a bucket is full:
    /// * and the bucket is a near bucket, it is split.
    /// * and is a far bucket or can no-longer be split, first bad nodes are
    ///   evicted, then and questionable nodes are queried. If any questionable
    ///   nodes turn out to be bad they are evicted.
    ///
    /// If there's no where to put a node, it is not added to the routing table.
    async fn add(&mut self, node_info: NodeInfo, transport: Arc<SendTransport>) {
        let bucket = self.find_bucket(node_info.node_id);
    }

    /// Finds the [`LeafNode`] which `node_id` will go into.
    fn find_bucket(&mut self, node_id: NodeID) -> &mut LeafNode {
        let mut bucket = &mut self.root;
        let mut bit_idx: usize = 0;

        loop {
            match bucket {
                Bucket::Leaf(leaf) => return leaf,
                Bucket::Inner(inner) => {
                    let bit = node_id.nth_bit(bit_idx);
                    bit_idx += 1;

                    if bit {
                        bucket = &mut inner.left
                    } else {
                        bucket = &mut inner.right
                    }
                }
            }
        }
    }

    fn find_node() {}

    fn find_nodes(_k_nearest: usize) {}
}

const K_BUCKET_SIZE: usize = 8;

enum Bucket {
    Leaf(LeafNode),
    Inner(InnerNode),
}

impl Bucket {
    fn split(&mut self) {
        unimplemented!()
    }

    fn add(&mut self, node_info: NodeInfo) {
        let leaf_node = match self {
            Bucket::Leaf(leaf) => leaf,
            Bucket::Inner(_) => assert!(false, "Bucket.add called on Bucket::Inner"),
        };

        unimplemented!()
    }
}

struct LeafNode {
    contacts: Vec<NodeContactState>,
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
    async fn add<'a>(
        &'a mut self,
        node_info: &NodeInfo,
        send_transport: &SendTransport,
    ) -> Option<&'a mut NodeContactState> {
        if let Some(existing) = self.find_node_mut_by_id(&node_info.node_id) {
            return Some(existing);
        };

        // space in bucket
        if self.contacts.len() <= K_BUCKET_SIZE {
            self.contacts.push(NodeContactState::new(
                node_info.node_id.clone(),
                node_info.address,
            ));
            return Some(&mut self.contacts[self.contacts.len() - 1]);
        }

        if let Some(node) = self.try_replace_bad_node(node_info) {
            return Some(node);
        }

        // TODO: Second Argument
        future::join_all(
            self.contacts
                .iter_mut()
                .filter(|node| node.state() == NodeState::Bad)
                .map(|node| self.ping_questionable_node(node, send_transport)),
        )
        .await;

        if let Some(existing) = self.find_node_mut_by_id(&node_info.node_id) {
            return Some(existing);
        };

        self.try_replace_bad_node(&node_info)
    }

    fn find_node_mut_by_id(&mut self, id: &NodeID) -> Option<&mut NodeContactState> {
        self.contacts.iter_mut().find(|n| n.id == id)
    }

    /// Finds the first bad node in this bucket and replaces it with a node
    /// created from [`new_node`].
    ///
    /// Returns the newly created node. If no bad nodes were in the bucket
    /// returns [`None`].
    fn try_replace_bad_node(&mut self, new_node: &NodeInfo) -> Option<&mut NodeContactState> {
        let maybe_bad_node = self
            .contacts
            .iter_mut()
            .enumerate()
            .find(|(idx, n)| n.state() == NodeState::Bad);

        if let Some((bad_node_idx, bad_node)) = maybe_bad_node {
            mem::replace(
                bad_node,
                NodeContactState::new(new_node.node_id.clone(), new_node.address),
            );
            return Some(&mut self.contacts[bad_node_idx]);
        }

        None
    }

    async fn ping_questionable_node(
        &self,
        node: &mut NodeContactState,
        send_transport: &SendTransport,
    ) {
        let response_result: Result<NodeID, Error> = send_transport
            .ping(node.id.clone(), SocketAddr::V4(node.address))
            .timeout(Duration::from_secs(3))
            .await
            .and_then(|f| f.into());

        match response_result {
            Ok(response) => node.mark_successful_query(),
            Err(err) => node.mark_failed_query(),
        }
    }

    // TODO: Wrap Send Transport Around Timeouts and Updating NodeContactState

    fn can_split(&self) -> bool {
        self.leaf_type == LeafType::Near
    }

    // TODO: Call This Method
    fn update_bucket_time(&mut self) -> impl Future<Item = ()> {
        self.last_updated = Utc::now().naive_utc();
        let (future, handle) = future::abortable(self.refresh_bucket());
        mem::replace(self.periodic_update_handle, handle).abort();

        future
    }

    async fn refresh_bucket(&mut self) {
        // TODO: Implement
        unimplemented!()
    }
}

// TODO: 0, 1 -> left, right

/// Non-leaf node in the K-Bucket tree.
struct InnerNode {
    left: Bucket,
    right: Bucket,
}

#[derive(PartialEq)]
enum LeafType {
    /// This bucket is near our node id. When it becomes too big, it is split.
    Near,

    /// This bucket is far from our node id. When it becomes too big, new nodes
    /// are ignored.
    Far,
}
