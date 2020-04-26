//! KRPC protocol built on top of `tokio`.
//!
//! # Send Only KRPC Node
//!
//! ```
//! use std::{net::SocketAddr, str::FromStr};
//! use futures::{future, StreamExt, TryStreamExt};
//! use tokio::{net::UdpSocket, runtime::current_thread::Runtime};
//! # use failure::Error;
//!
//! use tokio_krpc::KRPCNode;
//! use krpc_encoding::NodeID;
//!
//! # fn main() -> Result<(), Error> {
//! let bind_addr = SocketAddr::from_str("0.0.0.0:0")?;
//! let socket = UdpSocket::bind(&bind_addr)?;
//! let node = KRPCNode::new(socket);
//! let (send_transport, inbound_requests) = node.serve();
//!
//! let mut runtime = Runtime::new()?;
//! runtime.spawn(
//!     inbound_requests
//!         .map_err(|err| println!("Error in Inbound Requests: {}", err))
//!         .for_each(|_| future::ready(())),
//! );
//!
//! let bootstrap_node_addr = SocketAddr::from_str("67.215.246.10:6881")?;
//! let node_id = NodeID::random();
//! let response = runtime.block_on(send_transport.ping(node_id, bootstrap_node_addr))?;
//!
//! println!("{:?}", response);
//!
//! # Ok(())
//! # }
//! ```

// TODO: Not Sold on SendTransport Name
// TODO: Not Sold on KRPCNode Naming
// TODO: Write Docs for SendTransport
// TODO: Write Docs for KRPCNode
// TODO: Consider Moving Requests into Structs
// TODO: Consider Moving Responses + PortType into responses module
// TODO: Consider sharing response + request types between inbound and outbound
// TODO: Write Docs for responses module

mod active_transactions;
mod inbound;
mod inbound_query;
mod inbound_response_envelope;
mod krpc_node;
mod port_type;
pub mod recv_errors;
mod response_future;
pub mod responses;
pub mod send_errors;
mod send_transport;
mod transaction_id;

pub use self::{
    inbound_query::InboundQuery,
    krpc_node::KRPCNode,
    port_type::PortType,
    send_transport::SendTransport,
};
