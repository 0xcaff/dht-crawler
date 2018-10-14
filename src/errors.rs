use failure::Backtrace;
use failure::Context;
use failure::Fail;

use proto;
use std;
use std::fmt;
use std::net::SocketAddr;
use std::sync::PoisonError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(
        display = "Received an error message from peer: {}",
        protocol_error
    )]
    PeerError { protocol_error: proto::Error },

    #[fail(display = "Failed to parse response")]
    InvalidResponse,

    #[fail(display = "The response received wasn't the type expected for the request")]
    InvalidResponseType,

    #[fail(display = "The lock was poisoned")]
    LockPoisoned,

    #[fail(display = "Transaction not found. {}", transaction_id)]
    TransactionNotFound { transaction_id: u32 },

    #[fail(display = "Failed to encode request")]
    EncodeError,

    #[fail(display = "Failed to send to {}", to)]
    SendError { to: SocketAddr },

    #[fail(display = "Failed to bind")]
    BindError,

    #[fail(display = "Received IPv6 Address")]
    UnsupportedAddressTypeError,

    #[fail(display = "Unimplemented request type")]
    UnimplementedRequestType,

    #[fail(display = "Invalid Token")]
    InvalidToken,

    #[fail(display = "Insufficient address information provided.")]
    InsufficientAddress,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Error {
    pub fn as_request_error(&self) -> proto::Error {
        let (code, message) = match self.inner.get_context() {
            ErrorKind::UnimplementedRequestType => (204, "Unimplemented"),
            ErrorKind::InvalidToken => (203, "Invalid Token"),
            ErrorKind::InsufficientAddress => (203, "Not enough address info provided."),
            _ => (202, "Server Error"),
        };

        proto::Error::new(code, message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

/// Implementation allowing for converting to a `Fail` compatible error even when the lock isn't
/// sync.
impl<Guard> From<PoisonError<Guard>> for Error {
    fn from(_err: PoisonError<Guard>) -> Error {
        ErrorKind::LockPoisoned.into()
    }
}
