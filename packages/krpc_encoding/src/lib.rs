#![feature(error_generic_member_access, provide_any)]

//! Library for serializing and de-serializing krpc messages defined in
//! [BEP-0005].
//!
//! # Encode
//!
//! ```
//! use krpc_encoding::{Envelope, Message, Query};
//!
//! # fn main() -> krpc_encoding::errors::Result<()> {
//! let message = Envelope {
//!     ip: None,
//!     transaction_id: b"aa".to_vec(),
//!     version: None,
//!     message_type: Message::Query {
//!         query: Query::Ping {
//!             id: b"abcdefghij0123456789".into(),
//!         },
//!     },
//!     read_only: false,
//! };
//!
//! let encoded = message.encode()?;
//!
//! assert_eq!(
//!     encoded[..],
//!     b"d1:ad2:id20:abcdefghij0123456789e1:q4:ping1:t2:aa1:y1:qe"[..],
//! );
//! # Ok(())
//! # }
//! ```
//!
//! # Decode
//!
//! ```
//! use krpc_encoding::{Envelope, Query, Message};
//!
//! # fn main() -> krpc_encoding::errors::Result<()> {
//! let encoded = b"d1:ad2:id20:abcdefghij0123456789e1:q4:ping1:t2:aa1:y1:qe";
//!
//! assert_eq!(
//!     Envelope::decode(encoded)?,
//!     Envelope {
//!         ip: None,
//!         transaction_id: b"aa".to_vec(),
//!         version: None,
//!         message_type: Message::Query {
//!             query: Query::Ping {
//!                 id: b"abcdefghij0123456789".into(),
//!             },
//!         },
//!         read_only: false,
//!     },
//! );
//!
//! # Ok(())
//! # }
//! ```
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
        Envelope,
        KRPCError,
        Message,
        Query,
        Response,
    },
    node_id::{
        NodeID,
        NODE_ID_SIZE_BITS,
    },
    node_info::NodeInfo,
};
