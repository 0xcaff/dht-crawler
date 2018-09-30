use errors::Error;
use errors::ErrorKind;
use errors::Result;
use failure::ResultExt;

use proto;
use proto::Envelope;

use byteorder::NetworkEndian;
use byteorder::ReadBytesExt;

use std;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;

use std::sync::Arc;
use std::sync::Mutex;

use futures::task::Task;
use tokio;
use tokio::prelude::*;
use tokio::reactor::Handle;

use client::messages::{Request, Response, TransactionId};

type TransactionMap = HashMap<TransactionId, TxState>;

enum TxState {
    AwaitingResponse {
        /// Task to awake when response is received. None if poll hasn't been called for this tx
        /// yet.
        task: Option<Task>,
    },

    GotResponse {
        response: proto::Envelope,
    },
}

pub struct Peer {
    /// Socket used for sending messages
    send_socket: std::net::UdpSocket,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl Peer {
    pub fn new(bind_address: SocketAddr) -> Result<Peer> {
        let send_socket = std::net::UdpSocket::bind(&bind_address).context(ErrorKind::BindError)?;

        Ok(Peer {
            send_socket,
            transactions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn handle_responses(&self) -> Result<PeerFuture> {
        let raw_recv_socket = self.send_socket.try_clone().context(ErrorKind::BindError)?;
        let recv_socket = tokio::net::UdpSocket::from_std(raw_recv_socket, &Handle::default())
            .context(ErrorKind::BindError)?;

        Ok(PeerFuture {
            recv_socket,
            transactions: self.transactions.clone(),
        })
    }

    pub fn request(
        &self,
        address: SocketAddr,
        request: Request,
    ) -> impl Future<Item = Response, Error = Error> {
        let transaction_future = self.wait_for_response(request.transaction_id);

        self.send_request(address, request)
            .into_future()
            .and_then(move |_| transaction_future)
            .and_then(|envelope| Response::from(envelope))
    }

    /// Synchronously sends a request to `address`.
    ///
    /// The sending is done synchronously because doing it asynchronously was cumbersome and didn't
    /// make anything faster. UDP sending rarely blocks.
    fn send_request(&self, address: SocketAddr, request: Request) -> Result<()> {
        let transaction_id = request.transaction_id;
        let encoded = request.encode()?;

        self.send_socket
            .send_to(&encoded, &address)
            .with_context(|_| ErrorKind::SendError { to: address })?;

        self.transactions
            .lock()
            .map_err(|_| ErrorKind::LockPoisoned)
            .with_context(|_| ErrorKind::SendError { to: address })?
            .insert(transaction_id, TxState::AwaitingResponse { task: None });

        Ok(())
    }

    fn wait_for_response(&self, transaction_id: TransactionId) -> TransactionFuture {
        TransactionFuture {
            transaction_id,
            transactions: self.transactions.clone(),
        }
    }
}

/// A future which handles sending and receiving messages for the local peer.
pub struct PeerFuture {
    /// Socket for receiving messages from other peers
    recv_socket: tokio::net::UdpSocket,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl PeerFuture {
    fn handle_response(&self, buf: &[u8]) -> Result<()> {
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

impl Future for PeerFuture {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> std::result::Result<Async<Self::Item>, Self::Error> {
        let mut recv_buffer = [0 as u8; 1024];

        loop {
            try_ready!(self.recv_socket.poll_recv_from(&mut recv_buffer));
            self.handle_response(&recv_buffer).is_err();
        }
    }
}

/// A future which resolves when the response for a transaction appears in a peer's transaction map.
struct TransactionFuture {
    transaction_id: TransactionId,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<Mutex<TransactionMap>>,
}

impl Future for TransactionFuture {
    type Item = Envelope;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>> {
        let mut map = self
            .transactions
            .lock()
            .map_err(|_| ErrorKind::LockPoisoned)?;

        let tx_state = map.remove(&self.transaction_id);

        match tx_state {
            None => Err(ErrorKind::TransactionNotFound {
                transaction_id: self.transaction_id,
            })?,
            Some(tx_state @ TxState::AwaitingResponse { task: Some(..) }) => {
                map.insert(self.transaction_id, tx_state);
                Ok(Async::NotReady)
            }
            Some(TxState::AwaitingResponse { task: None }) => {
                let task = task::current();

                map.insert(
                    self.transaction_id,
                    TxState::AwaitingResponse { task: Some(task) },
                );

                Ok(Async::NotReady)
            }
            Some(TxState::GotResponse { response }) => Ok(Async::Ready(response)),
        }
    }
}
