use std::{
    backtrace::Backtrace,
    io,
};
use thiserror::Error;

// TODO: Review ErrorKinds
#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("failed to receive inbound message")]
    FailedToReceiveMessage {
        #[source]
        cause: io::Error,
    },

    #[error("invalid transaction id")]
    InvalidResponseTransactionId,

    #[error("failed to parse inbound message")]
    ParseInboundMessageError {
        #[source]
        cause: krpc_encoding::errors::Error,
    },

    #[error(
        "received response for an unknown transaction transaction_id={}",
        transaction_id
    )]
    UnknownTransactionReceived { transaction_id: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
#[error("{}", inner)]
pub struct Error {
    #[from]
    inner: ErrorKind,
    backtrace: Backtrace,
}
