use std;
use std::io;
use std::sync::{Arc, Mutex};

use byteorder::NetworkEndian;
use byteorder::ReadBytesExt;

use peer::peer::TransactionMap;
use proto::Envelope;

use errors::{ErrorKind, Result};
use failure::ResultExt;

use futures::task::Task;
use tokio;
use tokio::prelude::*;

pub enum TxState {
    AwaitingResponse {
        /// Task to awake when response is received. None if poll hasn't been called for this tx
        /// yet.
        task: Option<Task>,
    },

    GotResponse {
        response: Envelope,
    },
}

/// A future which handles receiving messages for the local peer.
pub struct InboundMessagesFuture {
    /// Socket for receiving messages from other peers
    recv_socket: tokio::net::UdpSocket,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl InboundMessagesFuture {
    pub fn new(
        recv_socket: tokio::net::UdpSocket,
        transactions: Arc<Mutex<TransactionMap>>,
    ) -> InboundMessagesFuture {
        InboundMessagesFuture {
            recv_socket,
            transactions,
        }
    }
}

impl InboundMessagesFuture {
    fn handle_inbound_message(&self, buf: &[u8]) -> Result<()> {
        let envelope = Envelope::decode(&buf).context(ErrorKind::InvalidResponse)?;

        let transaction_id = (&envelope.transaction_id[..])
            .read_u32::<NetworkEndian>()
            .context(ErrorKind::InvalidResponse)?;

        let mut map = self
            .transactions
            .lock()
            .map_err(|_| ErrorKind::LockPoisoned)?;

        let tx_state = map.remove(&transaction_id);

        let task = match tx_state {
            None => return Ok(()),
            Some(tx_state @ TxState::GotResponse { .. }) => {
                map.insert(transaction_id, tx_state);

                return Ok(());
            }
            Some(TxState::AwaitingResponse { task }) => task,
        };

        map.insert(transaction_id, TxState::GotResponse { response: envelope });

        if let Some(task) = task {
            task.notify();
        };

        Ok(())
    }
}

impl Future for InboundMessagesFuture {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> std::result::Result<Async<Self::Item>, Self::Error> {
        let mut recv_buffer = [0 as u8; 1024];

        loop {
            try_ready!(self.recv_socket.poll_recv_from(&mut recv_buffer));
            self.handle_inbound_message(&recv_buffer).is_err();
        }
    }
}
