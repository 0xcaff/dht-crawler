/*
use chrono::{
    DateTime,
    Utc,
};
use dht_crawler::{
    addr::{
        AsV4Address,
        IntoSocketAddr,
    },
    errors::Error as DhtError,
    proto::NodeID,
    stream::{
        run_forever,
        select_all,
    },
    transport::{
        RecvTransport,
        SendTransport,
    },
};
use failure::Error;
use futures::{
    stream,
    Future,
    Stream,
};
use serde::{
    Serialize,
    Serializer,
};
use serde_derive::Serialize;
use std::{
    iter,
    net::SocketAddrV4,
    sync::Arc,
    time::Duration,
};
use tokio::{
    runtime::Runtime,
    util::FutureExt,
};

fn main() -> Result<(), Error> {
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
            traverse(NodeID::random(), bootstrap_addr, None, send_transport_arc)
                .map(|node| println!("{}", serde_json::to_string(&node).unwrap()))
                .or_else(|e| {
                    eprintln!("Error While Traversing: {}", e);
                    Ok(())
                }),
        ))
        .ok();

    Ok(())
}

fn traverse(
    search_id: NodeID,
    addr: SocketAddrV4,
    from: Option<NodeID>,
    send_transport: Arc<SendTransport>,
) -> Box<dyn Stream<Item = Node, Error = DhtError> + Send> {
    Box::new(
        send_transport
            .find_node(search_id.clone(), addr.clone().into(), search_id.clone())
            .timeout(Duration::from_secs(5))
            .map_err(DhtError::from)
            .map(move |response| {
                let node = Node::new(response.id.clone(), from, search_id, addr);
                let from_id = response.id.clone();
                let result_stream = Box::new(stream::once(Ok(node)))
                    as Box<dyn Stream<Item = Node, Error = DhtError> + Send>;

                select_all(
                    iter::once(result_stream).chain(response.nodes.into_iter().map(|node| {
                        traverse(
                            NodeID::random(),
                            node.address,
                            Some(from_id.clone()),
                            send_transport.clone(),
                        )
                    })),
                )
            })
            .into_stream()
            .flatten(),
    )
}

#[derive(Serialize)]
struct Node {
    id: NodeIDWrapper,

    /// Node from which this node was discovered.
    from: Option<NodeIDWrapper>,

    /// Query which was made to `from` to find this node.
    query: NodeIDWrapper,

    /// Address of the node.
    address: SocketAddrV4,

    #[serde(serialize_with = "ts_milliseconds::serialize")]
    /// Time node was discovered.
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
    use chrono::{
        DateTime,
        Utc,
    };
    use serde::ser;

    pub fn serialize<S>(time: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_i64(time.timestamp_millis())
    }
}

impl Node {
    pub fn new(
        id: NodeID,
        discovered_from: Option<NodeID>,
        query: NodeID,
        address: SocketAddrV4,
    ) -> Node {
        Node {
            id: NodeIDWrapper(id),
            from: discovered_from.map(NodeIDWrapper),
            query: NodeIDWrapper(query),
            address,
            time_discovered: Utc::now(),
        }
    }
}
*/

fn main() {
    println!("hello world")
}
