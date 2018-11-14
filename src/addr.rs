use errors::{ErrorKind, Result};
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};

pub trait AsV4Address {
    fn into_v4(self) -> Result<SocketAddrV4>;
}

impl AsV4Address for SocketAddr {
    fn into_v4(self) -> Result<SocketAddrV4> {
        match self {
            SocketAddr::V4(addr) => Ok(addr),
            SocketAddr::V6(addr) => Err(ErrorKind::UnsupportedAddressTypeError { addr })?,
        }
    }
}

pub trait IntoSocketAddr {
    fn into_addr(self) -> SocketAddr;
}

impl<T> IntoSocketAddr for T
where
    T: ToSocketAddrs,
{
    fn into_addr(self) -> SocketAddr {
        self.to_socket_addrs().unwrap().nth(0).unwrap()
    }
}
