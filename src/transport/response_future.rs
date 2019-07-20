use crate::{
    errors::{
        Error,
        ErrorKind,
        Result,
    },
    proto::Message,
    transport::messages::{
        parse_originating_transaction_id,
        TransactionId,
    },
};
use futures::task::Task;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        Mutex,
    },
};
use tokio::prelude::*;

/// A future which resolves when the response for a transaction appears in a
/// peer's transaction map.
pub struct ResponseFuture {
    transaction_id: TransactionId,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl ResponseFuture {
    pub fn wait_for_tx(
        transaction_id: TransactionId,
        transactions: Arc<Mutex<TransactionMap>>,
    ) -> Result<ResponseFuture> {
        let fut = ResponseFuture::new(transaction_id, transactions);
        fut.add_to_tx_map()?;

        Ok(fut)
    }

    pub fn handle_response(
        message: Message,
        transactions: Arc<Mutex<TransactionMap>>,
    ) -> Result<()> {
        let transaction_id = parse_originating_transaction_id(&message.transaction_id[..])?;
        let mut map = transactions.lock()?;

        let tx_state = map
            .remove(&transaction_id)
            .ok_or_else(|| ErrorKind::UnknownTransaction { transaction_id })?;

        match tx_state {
            TxState::GotResponse { .. } => {
                map.insert(transaction_id, tx_state);
            }
            TxState::AwaitingResponse { task } => {
                map.insert(transaction_id, TxState::GotResponse { response: message });

                if let Some(task) = task {
                    task.notify();
                }
            }
        };

        Ok(())
    }

    fn add_to_tx_map(&self) -> Result<()> {
        let mut map = self.transactions.lock()?;

        map.insert(
            self.transaction_id,
            TxState::AwaitingResponse { task: None },
        );

        Ok(())
    }

    fn new(
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
    type Item = Message;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>> {
        let mut map = self.transactions.lock()?;

        let tx_state =
            map.remove(&self.transaction_id)
                .ok_or_else(|| ErrorKind::MissingTransactionState {
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

impl Drop for ResponseFuture {
    fn drop(&mut self) {
        let _ = self
            .transactions
            .lock()
            .map(|mut map| map.remove(&self.transaction_id));
    }
}

pub type TransactionMap = HashMap<TransactionId, TxState>;

pub enum TxState {
    AwaitingResponse {
        /// Task to awake when response is received. None if poll hasn't been
        /// called for this tx yet.
        task: Option<Task>,
    },

    GotResponse {
        response: Message,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        ResponseFuture,
        TxState,
    };
    use crate::errors::Error as DhtError;
    use failure::Error;
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            Mutex,
        },
    };

    #[test]
    fn test_drop() -> Result<(), Error> {
        let transaction_id = 0xafu32;
        let transactions = Arc::new(Mutex::new(HashMap::new()));

        {
            let _fut = ResponseFuture::wait_for_tx(transaction_id, transactions.clone())?;

            let transactions = transactions.lock().map_err(DhtError::from)?;
            let transaction = transactions.get(&transaction_id).unwrap();

            match transaction {
                TxState::AwaitingResponse { task: None } => assert!(true),
                _ => assert!(false),
            };
        }

        assert!(transactions
            .lock()
            .map_err(DhtError::from)?
            .get(&transaction_id)
            .is_none());

        Ok(())
    }
}
