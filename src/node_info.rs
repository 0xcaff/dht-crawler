use serde::Serialize;
use serde::Serializer;

use serde::de;
use serde::Deserialize;
use serde::Deserializer;

use std::net::SocketAddrV4;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use core::fmt;
use serde::de::Visitor;
use std::net::Ipv4Addr;

#[derive(Eq, PartialEq, Debug)]
pub struct NodeInfo(pub SocketAddrV4);

impl Serialize for NodeInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ip = self.0.ip();
        let port = self.0.port();
        let mut raw = [0u8; 6];

        raw[..4].clone_from_slice(&ip.octets());
        (&mut raw[4..])
            .write_u16::<NetworkEndian>(port)
            .expect("Failed to encode port.");

        serializer.serialize_bytes(&raw)
    }
}

impl<'de> Deserialize<'de> for NodeInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(NodeInfoVisitor)
    }
}

struct NodeInfoVisitor;

impl<'de> Visitor<'de> for NodeInfoVisitor {
    type Value = NodeInfo;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a byte array of size 6")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let ip = Ipv4Addr::new(v[0], v[1], v[2], v[3]);
        let port = (&v[4..]).read_u16::<NetworkEndian>().unwrap();

        Ok(NodeInfo(SocketAddrV4::new(ip, port)))
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_test;

    use self::serde_test::{assert_tokens, Token};
    use node_info::NodeInfo;
    use std::net::Ipv4Addr;
    use std::net::SocketAddrV4;

    #[test]
    fn serde() {
        let addr = Ipv4Addr::new(129, 21, 60, 66);
        let port = 12019;
        let socket_addr = SocketAddrV4::new(addr, port);

        let node_info = NodeInfo(socket_addr);

        assert_tokens(&node_info, &[Token::Bytes(&[129, 21, 60, 66, 0x2e, 0xf3])]);
    }
}
