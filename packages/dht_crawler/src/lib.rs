#![feature(async_await)]

//! Please note the terminology used in this library to avoid confusion. A
//! "peer" is a client/server listening on a TCP port that implements the
//! BitTorrent protocol. A "node" is a client/server listening on a UDP port
//! implementing the distributed hash table protocol. The DHT is composed of
//! nodes and stores the location of peers. BitTorrent clients include a DHT
//! node, which is used to contact other nodes in the DHT to get the location of
//! peers to download from using the BitTorrent protocol.

pub mod addr;
pub mod dht;
pub mod errors;
pub mod routing;

pub use crate::dht::Dht;
