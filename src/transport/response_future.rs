use crate::{
    errors::{
        Error,
        Result,
    },
    proto::Message,
    transport::{
        active_transactions::ActiveTransactions,
        messages::TransactionId,
    },
};

use tokio::prelude::*;

/// A future which resolves when the response for a transaction appears in a
/// peer's transaction map.
pub struct ResponseFuture {
    transaction_id: TransactionId,
    transactions: ActiveTransactions,
}

impl ResponseFuture {
    pub fn wait_for_tx(
        transaction_id: TransactionId,
        transactions: ActiveTransactions,
    ) -> Result<ResponseFuture> {
        transactions.add_transaction(transaction_id)?;
        let fut = ResponseFuture::new(transaction_id, transactions);

        Ok(fut)
    }

    fn new(transaction_id: TransactionId, transactions: ActiveTransactions) -> ResponseFuture {
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
        self.transactions.poll_response(self.transaction_id)
    }
}

impl Drop for ResponseFuture {
    fn drop(&mut self) {
        self.transactions
            .drop_transaction(self.transaction_id)
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        errors::Result,
        transport::{
            active_transactions::ActiveTransactions,
            response_future::ResponseFuture,
        },
    };

    #[test]
    fn test_drop() -> Result<()> {
        let transaction_id = 0xafu32;
        let transactions = ActiveTransactions::new();

        {
            let _fut = ResponseFuture::wait_for_tx(transaction_id, transactions.clone())?;
            assert!(transactions.contains_transaction(transaction_id)?);
        }

        assert!(!transactions.contains_transaction(transaction_id)?);

        Ok(())
    }
}
