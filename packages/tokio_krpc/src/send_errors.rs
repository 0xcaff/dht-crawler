use krpc_encoding as proto;
use std::{
    backtrace::Backtrace,
    io,
};
use thiserror::Error;

// TODO: Review ErrorKinds
#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("received an error message from node {}", error)]
    ReceivedKRPCError { error: proto::KRPCError },

    #[error("invalid response type, expected {} got {:?}", expected, got)]
    InvalidResponseType {
        expected: &'static str,
        got: krpc_encoding::Response,
    },

    #[error("failed to send")]
    SendError {
        #[source]
        cause: io::Error,
    },

    #[error("failed to encode message for sending")]
    SendEncodingError {
        #[source]
        cause: krpc_encoding::errors::Error,
    },

    #[error("transaction state missing for transaction_id={}", transaction_id)]
    UnknownTransactionPolled { transaction_id: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
#[error("{}", inner)]
pub struct Error {
    #[from]
    inner: ErrorKind,
    backtrace: Backtrace,
}
