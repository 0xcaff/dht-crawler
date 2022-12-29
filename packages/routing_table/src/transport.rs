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
                .map_err(|err| err.into())
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
    use failure::{
        Backtrace,
        Context,
        Fail,
    };
    use krpc_encoding::NodeID;
    use std::fmt;
    use tokio_krpc::send_errors;

    #[derive(Debug, Fail)]
    pub enum ErrorKind {
        #[fail(display = "failed to send query")]
        SendError {
            #[fail(cause)]
            cause: send_errors::Error,
        },

        #[fail(display = "node responded with unexpected id")]
        PingIdMismatch { got: NodeID, expected: NodeID },

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

    impl From<send_errors::Error> for Error {
        fn from(cause: send_errors::Error) -> Self {
            ErrorKind::SendError { cause }.into()
        }
    }
}
