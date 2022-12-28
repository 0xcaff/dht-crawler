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
    spawn,
};
use tokio_krpc::{
    KRPCNode,
    RequestTransport,
};

#[tokio::test]
async fn ping() -> Result<(), Error> {
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
    let socket = UdpSocket::bind(&bind).await?;
    let recv_transport = KRPCNode::new(socket);
    let (send_transport, request_stream) = recv_transport.serve();
    let request_transport = RequestTransport::new(id, send_transport);

    spawn(
        request_stream
            .map_err(|err| println!("Error in Request Stream: {}", err))
            .for_each(|_| future::ready(())),
    );

    let response = request_transport.ping(remote_v4).await?;

    assert_ne!(response, b"0000000000000000000000000000000000000000".into());

    Ok(())
}
