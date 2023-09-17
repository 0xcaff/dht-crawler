#![feature(error_generic_member_access)]

//! KRPC protocol built on top of `tokio`.
//!
//! # Send Only KRPC Node
//!
//! ```
//! use std::net::{SocketAddrV4, SocketAddr};
//! use std::error::Error;
//! use futures::{future, StreamExt, TryStreamExt};
//! use tokio::{net::UdpSocket};
//!
//! use tokio_krpc::{KRPCNode, RequestTransport};
//! use krpc_encoding::NodeID;
//! use tokio::spawn;
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     let bind_addr: SocketAddrV4 = "0.0.0.0:0".parse()?;
//!     let socket = UdpSocket::bind::<SocketAddrV4>(bind_addr).await?;
//!     let node = KRPCNode::new(socket);
//!     let node_id = NodeID::random();
//!     let (send_transport, inbound_requests) = node.serve();
//!     let request_transport = RequestTransport::new(node_id, send_transport);
//!
//!     spawn(
//!         inbound_requests
//!             .map_err(|err| println!("Error in Inbound Requests: {}", err))
//!             .for_each(|_| future::ready(())),
//!     );
//!
//!     let bootstrap_node_addr: SocketAddrV4 = "67.215.246.10:6881".parse()?;
//!     let response = request_transport.ping(bootstrap_node_addr).await?;
//!
//!     println!("{:?}", response);
//!
//!     Ok(())
//! }
//! ```

// TODO: Not Sold on SendTransport Name
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
mod request_transport;
mod response_future;
pub mod responses;
pub mod send_errors;
mod send_transport;
mod transaction_id;

pub use self::{
    inbound_query::InboundQuery,
    krpc_node::KRPCNode,
    port_type::PortType,
    request_transport::RequestTransport,
    send_transport::SendTransport,
};
