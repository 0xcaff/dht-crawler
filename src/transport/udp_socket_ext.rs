use futures::{
    Async,
    Future,
};
use std::net::SocketAddr;
use tokio_udp::UdpSocket;

use std::io;

pub trait UdpSocketExt {
    /// Returns a future that sends data on the socket to the given address.
    /// On success, the future will resolve to the number of bytes written.
    ///
    /// The future will resolve to an error if the IP version of the socket does
    /// not match that of `target`.
    fn send_to<'a, 'b>(&'a self, buf: &'b [u8], target: &'b SocketAddr) -> SendTo<'a, 'b>;
}

impl UdpSocketExt for UdpSocket {
    fn send_to<'a, 'b>(&'a self, buf: &'b [u8], target: &'b SocketAddr) -> SendTo<'a, 'b> {
        SendTo::new(self, buf, target)
    }
}

/// A future that sends a datagram to a given address.
///
/// This `struct` is created by [`send_to`](super::UdpSocket::send_to).
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub struct SendTo<'a, 'b> {
    socket: &'a UdpSocket,
    buf: &'b [u8],
    target: &'b SocketAddr,
}

impl<'a, 'b> SendTo<'a, 'b> {
    pub(super) fn new(socket: &'a UdpSocket, buf: &'b [u8], target: &'b SocketAddr) -> Self {
        Self {
            socket,
            buf,
            target,
        }
    }
}

impl<'a, 'b> Future for SendTo<'a, 'b> {
    type Item = usize;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, io::Error> {
        self.socket.poll_send_to(self.buf, self.target)
    }
}
