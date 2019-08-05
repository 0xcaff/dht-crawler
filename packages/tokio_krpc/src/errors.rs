use failure::{
    Backtrace,
    Context,
    Fail,
};
use krpc_encoding as proto;
use std::{
    fmt,
    io,
    net::SocketAddr,
};

// TODO: Review ErrorKinds
#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Received an error message from node {}", error)]
    ReceivedKRPCError { error: proto::KRPCError },

    #[fail(display = "Invalid response type, expected {} got {:?}", expected, got)]
    InvalidResponseType {
        expected: &'static str,
        got: krpc_encoding::Response,
    },

    #[fail(display = "Failed to bind")]
    BindError {
        #[fail(cause)]
        cause: io::Error,
    },

    #[fail(display = "failed to receive inbound message")]
    FailedToReceiveMessage {
        #[fail(cause)]
        cause: io::Error,
    },

    #[fail(display = "Invalid transaction id")]
    InvalidResponseTransactionId,

    #[fail(display = "Failed to parse inbound message from {}", from)]
    InvalidInboundMessage { from: SocketAddr, message: Vec<u8> },

    #[fail(display = "Failed to send to {}", to)]
    SendError { to: SocketAddr },

    #[fail(display = "Failed to encode message for sending")]
    SendEncodingError {
        #[fail(cause)]
        cause: krpc_encoding::errors::Error,
    },

    #[fail(display = "Failed to parse inbound message")]
    ParseInboundMessageError {
        #[fail(cause)]
        cause: krpc_encoding::errors::Error,
    },

    #[fail(
        display = "Transaction state missing for transaction_id={}",
        transaction_id
    )]
    UnknownTransactionPolled { transaction_id: u32 },

    #[fail(
        display = "Received response for an unknown transaction transaction_id={}",
        transaction_id
    )]
    UnknownTransactionReceived { transaction_id: u32 },
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
