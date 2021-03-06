//! Handle incoming responses and queries from other nodes.

use crate::recv_errors::{
    Error,
    ErrorKind,
    Result,
};
use futures::{
    stream,
    TryStream,
};
use krpc_encoding::Envelope;
use std::{
    self,
    net::SocketAddr,
};
use tokio::{
    self,
    net::udp::split::UdpSocketRecvHalf,
};

pub fn receive_inbound_messages(
    recv_socket: UdpSocketRecvHalf,
) -> impl TryStream<Ok = (Envelope, SocketAddr), Error = Error> {
    let recv_buffer = [0 as u8; 1024];

    stream::unfold((recv_socket, recv_buffer), |(recv_socket, recv_buffer)| {
        receive_inbound_message_wrapper(recv_socket, recv_buffer)
    })
}

async fn receive_inbound_message_wrapper(
    mut recv_socket: UdpSocketRecvHalf,
    mut recv_buffer: [u8; 1024],
) -> Option<(
    Result<(Envelope, SocketAddr)>,
    (UdpSocketRecvHalf, [u8; 1024]),
)> {
    let result = receive_inbound_message(&mut recv_socket, &mut recv_buffer).await;

    Some((result, (recv_socket, recv_buffer)))
}

async fn receive_inbound_message(
    recv_socket: &mut UdpSocketRecvHalf,
    recv_buffer: &mut [u8; 1024],
) -> Result<(Envelope, SocketAddr)> {
    let (size, from_addr) = recv_socket
        .recv_from(recv_buffer)
        .await
        .map_err(|cause| ErrorKind::FailedToReceiveMessage { cause })?;

    let envelope = Envelope::decode(&recv_buffer[..size])
        .map_err(|cause| ErrorKind::ParseInboundMessageError { cause })?;

    Ok((envelope, from_addr))
}
