use std;
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};

use byteorder::NetworkEndian;
use byteorder::ReadBytesExt;

use peer::messages::TransactionId;
use proto::{Envelope, MessageType};

use errors::{ErrorKind, Result};
use failure::ResultExt;

use futures::task::Task;
use tokio;
use tokio::prelude::*;

pub type TransactionMap = HashMap<TransactionId, TxState>;

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

        match envelope.message_type {
            MessageType::Error { .. } | MessageType::Response { .. } => {
                self.handle_response(envelope)
            }
            MessageType::Query { .. } => self.handle_request(envelope),
        }
    }

    fn handle_response(&self, envelope: Envelope) -> Result<()> {
        let transaction_id = (&envelope.transaction_id[..])
            .read_u32::<NetworkEndian>()
            .context(ErrorKind::InvalidResponse)?;

        let mut map = self
            .transactions
            .lock()
            .map_err(|_| ErrorKind::LockPoisoned)?;

        let tx_state = map
            .remove(&transaction_id)
            .ok_or_else(|| ErrorKind::TransactionNotFound { transaction_id })?;

        match tx_state {
            TxState::GotResponse { .. } => {
                map.insert(transaction_id, tx_state);
            }
            TxState::AwaitingResponse { task } => {
                map.insert(transaction_id, TxState::GotResponse { response: envelope });

                if let Some(task) = task {
                    task.notify();
                };
            }
        };

        Ok(())
    }

    fn handle_request(&self, envelope: Envelope) -> Result<()> {
        // TODO: Implement
        unimplemented!()
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
