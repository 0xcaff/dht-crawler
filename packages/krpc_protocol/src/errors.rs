use failure::{
    Backtrace,
    Context,
    Fail,
};
use serde_bencode::Error as BencodeError;
use std::fmt;

#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Error while encoding message")]
    EncodeError {
        #[fail(cause)]
        cause: BencodeError,
    },

    #[fail(display = "Error while decoding message")]
    DecodeError {
        #[fail(cause)]
        cause: BencodeError,
    },
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
