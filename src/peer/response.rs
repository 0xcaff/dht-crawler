use peer::inbound::{TransactionMap, TxState};
use peer::messages::TransactionId;
use proto::Envelope;

use errors::{Error, ErrorKind, Result};
use std::sync::{Arc, Mutex};
use tokio::prelude::*;

/// A future which resolves when the response for a transaction appears in a peer's transaction map.
pub struct ResponseFuture {
    transaction_id: TransactionId,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl ResponseFuture {
    pub fn new(
        transaction_id: TransactionId,
        transactions: Arc<Mutex<TransactionMap>>,
    ) -> ResponseFuture {
        ResponseFuture {
            transaction_id,
            transactions,
        }
    }
}

impl Future for ResponseFuture {
    type Item = Envelope;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>> {
        let mut map = self
            .transactions
            .lock()
            .map_err(|_| ErrorKind::LockPoisoned)?;

        let tx_state =
            map.remove(&self.transaction_id)
                .ok_or_else(|| ErrorKind::TransactionNotFound {
                    transaction_id: self.transaction_id,
                })?;

        Ok(match tx_state {
            TxState::AwaitingResponse { task: Some(..) } => {
                map.insert(self.transaction_id, tx_state);

                Async::NotReady
            }
            TxState::AwaitingResponse { task: None } => {
                let task = task::current();

                map.insert(
                    self.transaction_id,
                    TxState::AwaitingResponse { task: Some(task) },
                );

                Async::NotReady
            }
            TxState::GotResponse { response } => Async::Ready(response),
        })
    }
}
