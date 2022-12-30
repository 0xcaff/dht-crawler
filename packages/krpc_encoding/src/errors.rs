use serde_bencode::Error as BencodeError;
use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("error while encoding message")]
    EncodeError {
        #[source]
        cause: BencodeError,
    },

    #[error("error while decoding message")]
    DecodeError {
        #[source]
        cause: BencodeError,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[error("{}", inner)]
pub struct Error {
    #[from]
    inner: ErrorKind,

    backtrace: Backtrace,
}
