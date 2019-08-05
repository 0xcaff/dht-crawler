use crate::recv_errors::{
    ErrorKind,
    Result,
};
use byteorder::{
    NetworkEndian,
    ReadBytesExt,
};
use failure::ResultExt;

/// Transaction identifier used for requests originating from this client.
/// Requests originating from other clients use a `Vec<u8>` to represent the
/// transaction id.
pub type TransactionId = u32;

/// Extracts a [TransactionId] from a response to a request originating from
/// this client. If the transaction id is malformed, returns an error.
pub fn parse_originating_transaction_id(mut bytes: &[u8]) -> Result<TransactionId> {
    if bytes.len() != 4 {
        Err(ErrorKind::InvalidResponseTransactionId)?;
    }

    Ok(bytes
        .read_u32::<NetworkEndian>()
        .context(ErrorKind::InvalidResponseTransactionId)?)
}
