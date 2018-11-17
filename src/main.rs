extern crate chrono;
extern crate dht_crawler;
extern crate failure;
extern crate futures;
extern crate serde;
extern crate serde_json;
extern crate tokio;

#[macro_use]
extern crate serde_derive;

use failure::Error;
use std::{iter, net::SocketAddrV4, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use futures::{stream, Future, Stream};
use tokio::{runtime::Runtime, util::FutureExt};

use dht_crawler::{
    addr::{AsV4Address, IntoSocketAddr},
    errors::Error as DhtError,
    proto::NodeID,
    stream::{run_forever, select_all},
    transport::{RecvTransport, SendTransport},
};
use serde::Serialize;
use serde::Serializer;

fn main() -> Result<(), Error> {
    let node_id = NodeID::random();
    println!("Node ID: {:?}", node_id);

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
            traverse(node_id, None, bootstrap_addr, send_transport_arc)
                .map(|node| println!("{}", serde_json::to_string(&node).unwrap()))
                .or_else(|e| {
                    eprintln!("Error While Traversing: {}", e);
                    Ok(())
                }),
        )).ok();

    Ok(())
}

fn traverse(
    self_id: NodeID,
    from: Option<NodeID>,
    addr: SocketAddrV4,
    send_transport: Arc<SendTransport>,
) -> Box<Stream<Item = Node, Error = DhtError> + Send> {
    Box::new(
        send_transport
            .find_node(self_id.clone(), addr.clone().into(), self_id.clone())
            .timeout(Duration::from_secs(5))
            .map_err(DhtError::from)
            .map(move |response| {
                let node = Node::new(response.id.clone(), from, addr);
                let id = response.id.clone();
                let result_stream = Box::new(stream::once(Ok(node)))
                    as Box<dyn Stream<Item = Node, Error = DhtError> + Send>;

                select_all(
                    iter::once(result_stream).chain(response.nodes.into_iter().map(|node| {
                        traverse(
                            self_id.clone(),
                            Some(id.clone()),
                            node.address,
                            send_transport.clone(),
                        )
                    })),
                )
            }).into_stream()
            .flatten(),
    )
}

#[derive(Serialize)]
struct Node {
    id: NodeIDWrapper,
    from: Option<NodeIDWrapper>,
    address: SocketAddrV4,

    #[serde(serialize_with = "ts_milliseconds::serialize")]
    time_discovered: DateTime<Utc>,
}

struct NodeIDWrapper(NodeID);

impl Serialize for NodeIDWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

mod ts_milliseconds {
    use chrono::{DateTime, Utc};
    use serde::ser;

    pub fn serialize<S>(time: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_i64(time.timestamp_millis())
    }
}

impl Node {
    pub fn new(id: NodeID, discovered_from: Option<NodeID>, address: SocketAddrV4) -> Node {
        Node {
            id: NodeIDWrapper(id),
            from: discovered_from.map(NodeIDWrapper),
            address,
            time_discovered: Utc::now(),
        }
    }
}
