// TODO: Rename Package to krpc_encoding
// TODO: Add Code Sample
// TODO: Include Something About Nodes vs Peers

//! Library for serializing and de-serializing krpc messages defined in
//! [BEP-0005].
//!
//! [BEP-0005]: https://www.bittorrent.org/beps/bep_0005.html

mod addr;
mod booleans;
pub mod errors;
mod messages;
mod node_id;
mod node_info;

pub use self::{
    addr::{
        to_bytes as addr_to_bytes,
        Addr,
    },
    messages::{
        Message,
        MessageType,
        ProtocolError,
        Query,
        Response,
    },
    node_id::NodeID,
    node_info::NodeInfo,
};
