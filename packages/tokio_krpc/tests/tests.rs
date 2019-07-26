use byteorder::{
    NetworkEndian,
    WriteBytesExt,
};
use failure::Error;
use futures::{
    future,
    StreamExt,
    TryStreamExt,
};
use krpc_encoding::{
    self as proto,
    NodeID,
    Query,
};
use std::{
    net::{
        SocketAddr,
        ToSocketAddrs,
        UdpSocket,
    },
    str::FromStr,
};
use tokio::runtime::current_thread::Runtime;
use tokio_krpc::{
    messages::{
        Request,
        Response,
        TransactionId,
    },
    RecvTransport,
};

#[test]
fn test_ping() -> Result<(), Error> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let bootstrap_node = "router.bittorrent.com:6881";
    socket.connect(bootstrap_node)?;

    let transaction_id = 0x8aba;
    let mut req = Request {
        transaction_id: Vec::new(),
        version: None,
        query: Query::Ping {
            id: b"abcdefghij0123456789".into(),
        },
        read_only: false,
    };

    req.transaction_id
        .write_u32::<NetworkEndian>(transaction_id)?;

    let req_encoded = req.into().encode()?;
    socket.send(&req_encoded)?;

    let mut recv_buffer = [0 as u8; 1024];
    let size = socket.recv(&mut recv_buffer)?;

    let resp = Response::parse(&recv_buffer[0..size])?;

    assert_eq!(resp.transaction_id, transaction_id);

    Ok(())
}

fn make_async_request(
    remote_addr: &str,
    transaction_id: TransactionId,
    request: Request,
) -> Result<Response, Error> {
    let local_addr = SocketAddr::from_str("0.0.0.0:0")?;
    let bootstrap_node_addr = remote_addr.to_socket_addrs().unwrap().nth(0).unwrap();

    let mut runtime = Runtime::new()?;

    let recv_transport = RecvTransport::new(local_addr)?;
    let (send_transport, request_stream) = recv_transport.serve();

    let responses_future = request_stream
        .map_err(|e| println!("Error In Request Stream: {}", e))
        .for_each(|_| future::ready(()));

    let request_future = send_transport.request(bootstrap_node_addr, transaction_id, request);

    runtime.spawn(responses_future);
    let resp = runtime.block_on(request_future)?;

    Ok(resp)
}

#[test]
fn test_ping_async() -> Result<(), Error> {
    let transaction_id = 0xafda;

    let req = Request {
        transaction_id: Vec::new(),
        version: None,
        query: Query::Ping {
            id: b"abcdefghij0123456780".into(),
        },
        read_only: false,
    };

    let resp = make_async_request("router.bittorrent.com:6881", transaction_id, req)?;

    assert_eq!(resp.transaction_id, transaction_id);

    Ok(())
}

#[test]
fn test_find_node() -> Result<(), Error> {
    let transaction_id = 0x21312;

    let id: NodeID = b"abcdefghij0123456780".into();

    let req = Request {
        transaction_id: Vec::new(),
        version: None,
        query: Query::FindNode {
            id: id.clone(),
            target: id.clone(),
        },
        read_only: false,
    };

    let resp = make_async_request("router.bittorrent.com:6881", transaction_id, req)?;

    assert_eq!(resp.transaction_id, transaction_id);

    match resp.response {
        proto::Response::NextHop { nodes, .. } => assert!(!nodes.is_empty()),
        _ => assert!(false),
    };

    Ok(())
}

#[test]
fn simple_ping() -> Result<(), Error> {
    let bind = SocketAddr::from_str("0.0.0.0:0")?;
    let remote = "router.bittorrent.com:6881"
        .to_socket_addrs()
        .unwrap()
        .nth(0)
        .unwrap();

    let id = NodeID::random();
    let mut rt = Runtime::new()?;
    let recv_transport = RecvTransport::new(bind)?;
    let (send_transport, request_stream) = recv_transport.serve();

    rt.spawn(
        request_stream
            .map_err(|err| println!("Error in Request Stream: {}", err))
            .for_each(|_| future::ready(())),
    );

    let response = rt.block_on(send_transport.ping(id, remote))?;

    assert_ne!(response, b"0000000000000000000000000000000000000000".into());

    Ok(())
}
