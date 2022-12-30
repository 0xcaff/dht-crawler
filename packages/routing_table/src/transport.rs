use self::errors::Result;
use crate::{
    node_contact_state::NodeContactState,
    transport::errors::{
        Error,
        ErrorKind,
    },
};
use tokio_krpc::RequestTransport;

/// A transport used for communicating with other nodes which keeps liveness
/// information up to date.
pub struct LivenessTransport {
    request_transport: RequestTransport,
}

impl LivenessTransport {
    pub fn new(request_transport: RequestTransport) -> LivenessTransport {
        LivenessTransport { request_transport }
    }

    pub async fn ping(&self, node: &mut NodeContactState) -> Result<()> {
        Ok(node.update_from_result(
            self.request_transport
                .ping(node.address)
                .await
                .map_err(|err| ErrorKind::from(err).into())
                .and_then(|node_id| {
                    if node_id != node.id {
                        Err(Error::from(ErrorKind::PingIdMismatch {
                            got: node_id,
                            expected: node.id.clone(),
                        }))
                    } else {
                        Ok(())
                    }
                }),
        )?)
    }
}

mod errors {
    use krpc_encoding::NodeID;
    use std::{
        backtrace::Backtrace,
    };
    use thiserror::Error;
    use tokio_krpc::send_errors;

    #[derive(Debug, Error)]
    pub enum ErrorKind {
        #[error("failed to send query")]
        SendError {
            #[from]
            cause: send_errors::Error,
        },

        #[error("node responded with unexpected id")]
        PingIdMismatch { got: NodeID, expected: NodeID },

        #[error("request timed out")]
        Timeout,
    }

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Error, Debug)]
    #[error("{}", inner)]
    pub struct Error {
        #[from]
        inner: ErrorKind,

        backtrace: Backtrace,
    }
}
