use errors::{ErrorKind, Result};
use std::net::{SocketAddr, SocketAddrV4};

pub trait AsV4Address {
    fn into_v4(self) -> Result<SocketAddrV4>;
}

impl AsV4Address for SocketAddr {
    fn into_v4(self) -> Result<SocketAddrV4> {
        match self {
            SocketAddr::V4(addr) => Ok(addr),
            SocketAddr::V6(..) => Err(ErrorKind::UnsupportedAddressTypeError)?,
        }
    }
}
