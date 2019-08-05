use tokio_krpc::SendTransport;
use krpc_encoding::NodeID;
use crate::node_contact_state::NodeContactState;

/// A [`SendTransport`] with a fixed timeout which updates a
/// [`NodeContactState`] when making requests.
struct RoutedSendTransport {
    send_transport: SendTransport,
}

impl RoutedSendTransport {
    fn new(send_transport: SendTransport) -> RoutedSendTransport {
        RoutedSendTransport {
            send_transport
        }
    }

    async fn ping(node: &mut NodeContactState) -> Result<NodeID> {

    }
}
