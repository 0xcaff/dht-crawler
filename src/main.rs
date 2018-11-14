extern crate dht_crawler;
extern crate failure;
extern crate futures;
extern crate tokio;

use failure::Error;
use std::{
    iter,
    net::SocketAddrV4,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{stream, Future, Stream};
use tokio::runtime::Runtime;
use tokio::util::FutureExt;

use dht_crawler::addr::AsV4Address;
use dht_crawler::addr::IntoSocketAddr;
use dht_crawler::errors::Error as DhtError;
use dht_crawler::proto::NodeID;
use dht_crawler::stream::run_forever;
use dht_crawler::stream::select_all;
use dht_crawler::transport::RecvTransport;
use dht_crawler::transport::SendTransport;

fn main() -> Result<(), Error> {
    let node_id = NodeID::random();
    let bind_addr = "0.0.0.0:21130".into_addr();
    let bootstrap_addr = "router.bittorrent.com:6881".into_addr().into_v4()?;

    let transport = RecvTransport::new(bind_addr)?;
    let (send_transport, request_stream) = transport.serve_read_only();

    let mut runtime = Runtime::new()?;
    runtime.spawn(run_forever(request_stream.map(|_| ()).or_else(|err| {
        eprintln!("Error While Handling Requests: {}", err);

        Ok(())
    })));

    let send_transport_arc = Arc::new(send_transport);
    runtime
        .block_on(run_forever(
            traverse(node_id, bootstrap_addr, send_transport_arc)
                .map(|node| println!("Node Discovered: {:#?}", node))
                .or_else(|e| {
                    eprintln!("Error While Traversing: {}", e);
                    Ok(())
                }),
        )).ok();

    Ok(())
}

fn traverse(
    self_id: NodeID,
    addr: SocketAddrV4,
    send_transport: Arc<SendTransport>,
) -> Box<Stream<Item = Node, Error = DhtError> + Send> {
    Box::new(
        send_transport
            .find_node(self_id.clone(), addr.clone().into(), self_id.clone())
            .timeout(Duration::from_secs(5))
            .map_err(DhtError::from)
            .map(move |response| {
                let node = Node::new(response.id, addr);
                let result_stream = Box::new(stream::once(Ok(node)))
                    as Box<dyn Stream<Item = Node, Error = DhtError> + Send>;

                select_all(iter::once(result_stream).chain(
                    response.nodes.into_iter().map(|node| {
                        traverse(self_id.clone(), node.address, send_transport.clone())
                    }),
                ))
            }).into_stream()
            .flatten(),
    )
}

#[derive(Debug)]
struct Node {
    id: NodeID,
    address: SocketAddrV4,
    time_discovered: Instant,
}

impl Node {
    pub fn new(id: NodeID, address: SocketAddrV4) -> Node {
        Node {
            id,
            address,
            time_discovered: Instant::now(),
        }
    }
}
