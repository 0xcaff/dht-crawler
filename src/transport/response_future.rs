use crate::{
    errors::{
        Error,
        Result,
    },
    proto,
    transport::{
        active_transactions::ActiveTransactions,
        messages::TransactionId,
    },
};
use futuresx::compat::Future01CompatExt;
use tokio::prelude::*;

/// A future which resolves when the response for a transaction appears in a
/// peer's transaction map.
pub struct ResponseFuture {
    transaction_id: TransactionId,
    transactions: ActiveTransactions,
}

impl ResponseFuture {
    pub async fn wait_for_tx(
        transaction_id: TransactionId,
        transactions: ActiveTransactions,
    ) -> Result<proto::Message> {
        transactions.add_transaction(transaction_id)?;

        let message = ResponseFuture::new(transaction_id, transactions)
            .compat()
            .await?;

        Ok(message)
    }

    fn new(transaction_id: TransactionId, transactions: ActiveTransactions) -> ResponseFuture {
        ResponseFuture {
            transaction_id,
            transactions,
        }
    }
}

impl Future for ResponseFuture {
    type Item = proto::Message;
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
