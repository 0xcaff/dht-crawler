use futures_util::{
    future,
    TryStreamExt,
    StreamExt,
};
use krpc_encoding::NodeID;
use routing_table::RoutingTable;
use std::{
    error::Error,
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
async fn bootstrap() -> Result<(), Box<dyn Error>> {
    let id = NodeID::random();

    let remote = "router.bittorrent.com:6881"
        .to_socket_addrs()
        .unwrap()
        .nth(0)
        .unwrap();

    let remote_v4 = match remote {
        SocketAddr::V4(v4) => v4,
        SocketAddr::V6(_) => panic!("not v4"),
    };

    let socket = UdpSocket::bind(SocketAddr::from_str("0.0.0.0:0")?).await?;

    let node = KRPCNode::new(socket);
    let (send_transport, request_stream) = node.serve();
    let request_transport = RequestTransport::new(id.clone(), send_transport);

    spawn(
        request_stream
            .map_err(|err| println!("Error in Request Stream: {}", err))
            .for_each(|_| future::ready(())),
    );

    let mut routing_table = RoutingTable::new(id, request_transport);

    routing_table.bootstrap(remote_v4).await;

    Ok(())
}
