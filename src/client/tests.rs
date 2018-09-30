use client::messages::{Request, Response};
use client::peer::Peer;

use proto;
use proto::Query;

use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::net::UdpSocket;
use std::str::FromStr;

use tokio::runtime::Runtime;

use futures::Future;

#[test]
fn test_ping() {
    let socket = UdpSocket::bind("0.0.0.0:34254").unwrap();
    let bootstrap_node = "router.bittorrent.com:6881";
    socket.connect(bootstrap_node).unwrap();

    let transaction_id = 0x8aba;
    let req = Request {
        transaction_id,
        version: None,
        query: Query::Ping {
            id: b"abcdefghij0123456789".into(),
        },
    };

    let req_encoded = req.encode().unwrap();
    socket.send(&req_encoded).unwrap();

    let mut recv_buffer = [0 as u8; 1024];
    socket.recv(&mut recv_buffer).unwrap();

    let resp = Response::parse(&recv_buffer).unwrap();

    assert_eq!(resp.transaction_id, transaction_id);
}

fn make_async_request(bind_addr: &str, remote_addr: &str, request: Request) -> Response {
    let local_addr = SocketAddr::from_str(bind_addr).unwrap();
    let bootstrap_node_addr = remote_addr.to_socket_addrs().unwrap().next().unwrap();

    let peer = Peer::new(local_addr).unwrap();
    let mut runtime = Runtime::new().unwrap();
    let responses_future = peer.handle_responses().unwrap().map_err(|_e| ());
    let request_future = peer.request(bootstrap_node_addr, request);

    runtime.spawn(responses_future);
    let resp = runtime.block_on(request_future).unwrap();
    runtime.shutdown_on_idle();

    resp
}

#[test]
fn test_ping_async() {
    let transaction_id = 0xafda;

    let req = Request {
        transaction_id,
        version: None,
        query: Query::Ping {
            id: b"abcdefghij0123456780".into(),
        },
    };

    let resp = make_async_request("0.0.0.0:34258", "router.bittorrent.com:6881", req);

    assert_eq!(resp.transaction_id, transaction_id)
}

#[test]
fn test_find_node() {
    let transaction_id = 0x21312;

    let id = b"abcdefghij0123456780".into();

    let req = Request {
        transaction_id,
        version: None,
        query: Query::FindNode {
            id,
            target: id.clone(),
        },
    };

    let resp = make_async_request("0.0.0.0:34218", "router.bittorrent.com:6881", req);

    assert_eq!(resp.transaction_id, transaction_id);

    match resp.response {
        proto::Response::NextHop { nodes, .. } => assert!(!nodes.is_empty()),
        _ => assert!(false),
    };
}
