use failure::Error;
use futures::{
    future,
    StreamExt,
    TryStreamExt,
};
use krpc_encoding::NodeID;
use std::{
    net::{
        SocketAddr,
        ToSocketAddrs,
    },
    str::FromStr,
};
use tokio::{
    net::UdpSocket,
    runtime::current_thread::Runtime,
};
use tokio_krpc::{
    KRPCNode,
    RequestTransport,
};

#[test]
fn ping() -> Result<(), Error> {
    let bind = SocketAddr::from_str("0.0.0.0:0")?;
    let remote = "router.bittorrent.com:6881"
        .to_socket_addrs()
        .unwrap()
        .nth(0)
        .unwrap();

    let remote_v4 = match remote {
        SocketAddr::V4(v4) => v4,
        SocketAddr::V6(_) => panic!("not v4"),
    };

    let id = NodeID::random();
    let mut rt = Runtime::new()?;
    let socket = UdpSocket::bind(&bind)?;
    let recv_transport = KRPCNode::new(socket);
    let (send_transport, request_stream) = recv_transport.serve();
    let request_transport = RequestTransport::new(send_transport);

    rt.spawn(
        request_stream
            .map_err(|err| println!("Error in Request Stream: {}", err))
            .for_each(|_| future::ready(())),
    );

    let response = rt.block_on(request_transport.ping(id, remote_v4))?;

    assert_ne!(response, b"0000000000000000000000000000000000000000".into());

    Ok(())
}
