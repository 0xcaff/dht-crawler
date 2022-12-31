use self::errors::Result;
use crate::{
    node_contact_state::NodeContactState,
    transport::errors::{
        Error,
        ErrorKind,
        TimeoutExt,
    },
};
use krpc_encoding::NodeID;
use std::net::SocketAddrV4;
use tokio_krpc::{
    responses::FindNodeResponse,
    RequestTransport,
};

/// A transport used for communicating with other nodes which keeps liveness
/// information up to date.
pub struct LivenessTransport {
    request_transport: RequestTransport,
}

impl LivenessTransport {
    pub fn new(request_transport: RequestTransport) -> LivenessTransport {
        LivenessTransport { request_transport }
    }

    pub async fn find_node(
        &self,
        address: SocketAddrV4,
        target: NodeID,
    ) -> Result<FindNodeResponse> {
        Ok(self
            .request_transport
            .find_node(address, target)
            .timeout()
            .await?)
    }

    pub async fn ping(&self, node: &mut NodeContactState) -> Result<()> {
        Ok(node.update_from_result(
            self.request_transport
                .ping(node.address)
                .timeout()
                .await
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
    use futures_util::{
        future::Map,
        FutureExt,
    };
    use krpc_encoding::NodeID;
    use std::{
        backtrace::Backtrace,
        future::Future,
        time::Duration,
    };
    use thiserror::Error;
    use tokio::time::{
        error::Elapsed,
        timeout,
        Timeout,
    };
    use tokio_krpc::{
        send_errors,
        send_errors::Result as TokioKrpcSendResult,
    };

    type WithTimeoutFuture<T, F> = Map<
        Timeout<F>,
        fn(
            std::result::Result<std::result::Result<T, tokio_krpc::send_errors::Error>, Elapsed>,
        ) -> Result<T>,
    >;

    fn with_timeout<T, F>(future: F, duration: Duration) -> WithTimeoutFuture<T, F>
    where
        F: Future<Output = TokioKrpcSendResult<T>>,
    {
        timeout(duration, future).map(|result| match result {
            Err(_cause) => Err(ErrorKind::Timeout.into()),
            Ok(Err(cause)) => Err(ErrorKind::SendError { cause }.into()),
            Ok(Ok(value)) => Ok(value),
        })
    }

    pub trait TimeoutExt<T>: Future<Output = TokioKrpcSendResult<T>> + Sized {
        fn timeout(self) -> WithTimeoutFuture<T, Self>;
    }

    const TIMEOUT_SECONDS: u64 = 1;

    impl<T, F> TimeoutExt<T> for F
    where
        F: Future<Output = TokioKrpcSendResult<T>> + Sized,
    {
        fn timeout(self) -> WithTimeoutFuture<T, Self> {
            with_timeout(self, Duration::new(TIMEOUT_SECONDS, 0))
        }
    }

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
