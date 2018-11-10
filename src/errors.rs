use failure::Backtrace;
use failure::Context;
use failure::Fail;

use proto;
use std;
use std::fmt;
use std::net::SocketAddr;
use std::sync::PoisonError;

use std::net::SocketAddrV6;
use tokio::timer::timeout;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Debug, Fail)]
pub enum ErrorKind {
    //// Originating Errors
    #[fail(
        display = "Received an error message from node {}",
        error_message
    )]
    ProtocolError { error_message: proto::Error },

    #[fail(display = "Failed to parse inbound message from {}", from)]
    InvalidInboundMessage { from: SocketAddr, message: Vec<u8> },

    #[fail(display = "Invalid transaction id")]
    InvalidResponseTransactionId,

    #[fail(
        display = "Transaction state missing for transaction_id={}",
        transaction_id
    )]
    MissingTransactionState { transaction_id: u32 },

    #[fail(
        display = "Received response for unknown transaction transaction_id={}",
        transaction_id
    )]
    UnknownTransaction { transaction_id: u32 },

    #[fail(display = "Received IPv6 Address where an IPv4 address was expected")]
    UnsupportedAddressTypeError { addr: SocketAddrV6 },

    #[fail(
        display = "Invalid message type, expected {} got {:?}",
        expected,
        got
    )]
    InvalidMessageType {
        expected: &'static str,
        got: proto::MessageType,
    },

    #[fail(
        display = "Invalid response type, expected {} got {:?}",
        expected,
        got
    )]
    InvalidResponseType {
        expected: &'static str,
        got: proto::Response,
    },

    #[fail(display = "Failed to bind")]
    BindError,

    #[fail(display = "Failed to send to {}", to)]
    SendError { to: SocketAddr },

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

    #[fail(display = "Error while encoding message")]
    EncodeError,

    #[fail(display = "Error while decoding message")]
    DecodeError,

    #[fail(display = "Timer error")]
    TimerError,

    #[fail(display = "Timeout")]
    Timeout,
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
            ErrorKind::InsufficientAddress => (203, "Not enough address info provided"),
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

impl From<timeout::Error<Error>> for Error {
    fn from(err: timeout::Error<Error>) -> Self {
        if err.is_inner() {
            return err.into_inner().unwrap();
        }

        if err.is_timer() {
            return err
                .into_timer()
                .unwrap()
                .context(ErrorKind::TimerError)
                .into();
        }

        ErrorKind::Timeout.into()
    }
}
