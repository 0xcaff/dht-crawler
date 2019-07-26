use crate::{
    errors::{
        ErrorKind,
        Result,
    },
    transport::messages::{
        parse_originating_transaction_id,
        TransactionId,
    },
};
use krpc_protocol as proto;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        Mutex,
    },
};
use tokio::prelude::{
    task::Waker,
    Poll,
};

/// A thread-safe container for information about active transactions. Shared
/// between many [`ResponseFuture`]s and a single [`RecvTransport`].
#[derive(Clone)]
pub struct ActiveTransactions {
    transactions: Arc<Mutex<HashMap<TransactionId, TxState>>>,
}

enum TxState {
    GotResponse {
        response: proto::Envelope,
    },
    AwaitingResponse {
        /// Waker used when response is received. None if poll hasn't been
        /// called for this tx yet.
        waker: Option<Waker>,
    },
}

impl ActiveTransactions {
    pub fn new() -> ActiveTransactions {
        let transactions = Arc::new(Mutex::new(HashMap::new()));

        ActiveTransactions { transactions }
    }

    /// Adds an un-polled pending transaction to the set of active transactions.
    pub(super) fn add_transaction(&self, transaction_id: TransactionId) -> Result<()> {
        let mut map = self.transactions.lock()?;
        map.insert(transaction_id, TxState::AwaitingResponse { waker: None });
        Ok(())
    }

    /// Stops tracking a transaction. Subsequent calls to [`handle_response`],
    /// [`poll_response`]  with `transaction_id` will now fail.
    pub(super) fn drop_transaction(&self, transaction_id: TransactionId) -> Result<()> {
        let mut map = self.transactions.lock()?;
        map.remove(&transaction_id);
        Ok(())
    }

    /// Updates transaction associated with `message` such that the next call to
    /// [`poll_response`] for the transaction will return [`Async::Ready`].
    ///
    /// # Errors
    ///
    /// If the transaction id associated with `message` isn't known, returns
    /// failure.
    pub(super) fn handle_response(&self, message: proto::Envelope) -> Result<()> {
        let transaction_id = parse_originating_transaction_id(&message.transaction_id)?;
        let mut map = self.transactions.lock()?;

        let current_tx_state = map
            .remove(&transaction_id)
            .ok_or_else(|| ErrorKind::UnknownTransaction { transaction_id })?;

        match current_tx_state {
            TxState::GotResponse { .. } => {
                // Multiple responses received for a single transaction. This shouldn't happen.
                map.insert(transaction_id, current_tx_state);
            }
            TxState::AwaitingResponse { waker } => {
                map.insert(transaction_id, TxState::GotResponse { response: message });
                waker.map(|waker| waker.wake());
            }
        };

        Ok(())
    }

    /// Returns [`NotReady`] until a message with the same `transaction_id` is
    /// provided to [`handle_response`], then returns that message.
    ///
    /// # Panics
    ///
    /// This function calls [`Task::current`] which panics when called when a
    /// task is not currently being executed.
    pub(super) fn poll_response(
        &self,
        transaction_id: TransactionId,
        waker: &Waker,
    ) -> Poll<Result<proto::Envelope>> {
        let mut map = self.transactions.lock()?;

        let tx_state = map
            .remove(&transaction_id)
            .ok_or_else(|| ErrorKind::MissingTransactionState { transaction_id })?;

        match tx_state {
            TxState::GotResponse { response } => Poll::Ready(Ok(response)),
            TxState::AwaitingResponse { waker: Some(..) } => {
                map.insert(transaction_id, tx_state);

                Poll::Pending
            }
            TxState::AwaitingResponse { waker: None } => {
                map.insert(
                    transaction_id,
                    TxState::AwaitingResponse {
                        waker: Some(waker.clone()),
                    },
                );

                Poll::Pending
            }
        }
    }
}
