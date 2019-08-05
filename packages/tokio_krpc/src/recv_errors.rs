use failure::{
    Backtrace,
    Context,
    Fail,
};
use std::{
    fmt,
    io,
};

// TODO: Review ErrorKinds
#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "failed to receive inbound message")]
    FailedToReceiveMessage {
        #[fail(cause)]
        cause: io::Error,
    },

    #[fail(display = "Invalid transaction id")]
    InvalidResponseTransactionId,

    #[fail(display = "Failed to parse inbound message")]
    ParseInboundMessageError {
        #[fail(cause)]
        cause: krpc_encoding::errors::Error,
    },

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
