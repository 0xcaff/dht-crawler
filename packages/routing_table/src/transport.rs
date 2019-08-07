use self::errors::Result;
use crate::{
    node_contact_state::NodeContactState,
    transport::errors::ErrorKind,
};
use failure::_core::time::Duration;
use krpc_encoding::NodeID;
use std::net::SocketAddr;
use tokio::prelude::FutureExt;
use tokio_krpc::SendTransport;

pub struct WrappedSendTransport {
    send_transport: SendTransport,
}

impl WrappedSendTransport {
    pub async fn ping(&self, node: &mut NodeContactState) -> Result<NodeID> {
        let result: Result<NodeID> = self.ping_inner(node).await;

        match result {
            Ok(_) => node.mark_successful_query(),
            Err(_) => node.mark_failed_query(),
        };

        result
    }

    async fn ping_inner(&self, node: &mut NodeContactState) -> Result<NodeID> {
        Ok(self
            .send_transport
            .ping(node.id.clone(), SocketAddr::V4(node.address))
            .timeout(Duration::from_secs(3))
            .await?
            .map_err(|cause| ErrorKind::SendError { cause })?)
    }
}

mod errors {
    use failure::{
        Backtrace,
        Context,
        Fail,
    };
    use std::fmt;
    use tokio_krpc::send_errors;

    #[derive(Debug, Fail)]
    pub enum ErrorKind {
        #[fail(display = "failed to send query")]
        SendError {
            #[fail(cause)]
            cause: send_errors::Error,
        },

        #[fail(display = "request timed out")]
        Timeout,
    }

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug)]
    pub struct Error {
        inner: Context<ErrorKind>,
    }

    impl Fail for Error {
        fn cause(&self) -> Option<&dyn Fail> {
            self.inner.cause()
        }

        fn backtrace(&self) -> Option<&Backtrace> {
            self.inner.backtrace()
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.inner, f)
        }
    }

    impl From<ErrorKind> for Error {
        fn from(kind: ErrorKind) -> Error {
            Error {
                inner: Context::new(kind),
            }
        }
    }

    impl From<Context<ErrorKind>> for Error {
        fn from(inner: Context<ErrorKind>) -> Error {
            Error { inner }
        }
    }

    impl From<tokio::timer::timeout::Elapsed> for Error {
        fn from(_: tokio::timer::timeout::Elapsed) -> Self {
            ErrorKind::Timeout.into()
        }
    }
}
