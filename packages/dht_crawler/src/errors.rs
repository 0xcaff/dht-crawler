use failure::{
    Backtrace,
    Context,
    Fail,
};
use krpc_encoding as proto;
use std::{
    self,
    fmt,
    net::{
        SocketAddrV6,
    },
    sync::PoisonError,
};
use tokio::timer::timeout;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Debug, Fail)]
pub enum ErrorKind {
    //// Originating Errors
    #[fail(display = "Received IPv6 Address where an IPv4 address was expected")]
    UnsupportedAddressTypeError { addr: SocketAddrV6 },

    //// Protocol Errors
    #[fail(display = "Unimplemented request type")]
    UnimplementedRequestType,

    #[fail(display = "Invalid Token")]
    InvalidToken,

    #[fail(display = "Insufficient address information provided")]
    InsufficientAddress,

    //// Wrapping Other Errors
    #[fail(display = "Lock poisoned")]
    LockPoisoned,

    #[fail(display = "Timeout")]
    Timeout,

    #[fail(display = "Something broke in the transport")]
    TransportError {
        #[fail(cause)]
        cause: tokio_krpc::errors::Error,
    },
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Error {
    pub fn as_request_error(&self) -> proto::KRPCError {
        let (code, message) = match self.inner.get_context() {
            ErrorKind::UnimplementedRequestType => (204, "Unimplemented"),
            ErrorKind::InvalidToken => (203, "Invalid Token"),
            ErrorKind::InsufficientAddress => (203, "Not enough address info provided"),
            _ => (202, "Server Error"),
        };

        proto::KRPCError::new(code, message)
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

/// Implementation allowing for converting to a `Fail` compatible error even
/// when the lock isn't sync.
impl<Guard> From<PoisonError<Guard>> for Error {
    fn from(_err: PoisonError<Guard>) -> Error {
        ErrorKind::LockPoisoned.into()
    }
}

impl From<timeout::Elapsed> for Error {
    fn from(_err: timeout::Elapsed) -> Self {
        ErrorKind::Timeout.into()
    }
}

impl From<tokio_krpc::errors::Error> for Error {
    fn from(cause: tokio_krpc::errors::Error) -> Self {
        ErrorKind::TransportError { cause }.into()
    }
}
