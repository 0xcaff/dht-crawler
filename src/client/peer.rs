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
use std::sync::RwLock;

use tokio;
use tokio::prelude::*;
use tokio::reactor::Handle;

use client::messages::{Request, TransactionId};

type TransactionMap = HashMap<TransactionId, Option<proto::Envelope>>;

pub struct Peer {
    /// Socket used for sending messages
    send_socket: std::net::UdpSocket,

    /// Collection of in-flight transactions awaiting a response
    transactions: Arc<RwLock<TransactionMap>>,
}

impl Peer {
    pub fn new(bind_address: SocketAddr) -> Result<Peer> {
        let send_socket = std::net::UdpSocket::bind(&bind_address).context(ErrorKind::BindError)?;

        Ok(Peer {
            send_socket,
            transactions: Arc::new(RwLock::new(HashMap::new())),
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
    ) -> impl Future<Item = Envelope, Error = Error> {
        let transaction_future = self.wait_for_response(request.transaction_id);

        self.send_request(address, request)
            .into_future()
            .and_then(move |_| transaction_future)
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
            .write()
            .map_err(|_| ErrorKind::LockPoisoned)
            .with_context(|_| ErrorKind::SendError { to: address })?
            .insert(transaction_id, None);

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
    transactions: Arc<RwLock<TransactionMap>>,
}

impl PeerFuture {
    fn handle_response(&self, buf: &[u8]) -> Result<()> {
        let envelope = Envelope::decode(&buf).context(ErrorKind::InvalidResponse)?;

        let transaction_id = (&envelope.transaction_id[..])
            .read_u32::<NetworkEndian>()
            .context(ErrorKind::InvalidResponse)?;

        self.transactions
            .write()
            .map_err(|_| ErrorKind::LockPoisoned)?
            .get_mut(&transaction_id)
            .map(|r| r.get_or_insert(envelope));

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
    transactions: Arc<RwLock<TransactionMap>>,
}

impl Future for TransactionFuture {
    type Item = Envelope;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>> {
        println!("ASDF");

        let transaction_exists = self
            .transactions
            .read()
            .map_err(|_| ErrorKind::LockPoisoned)?
            .get(&self.transaction_id)
            .ok_or_else(|| ErrorKind::TransactionNotFound {
                transaction_id: self.transaction_id,
            })?.is_some();

        Ok(match transaction_exists {
            true => {
                let envelope = self
                    .transactions
                    .write()
                    .map_err(|_| ErrorKind::LockPoisoned)?
                    .remove(&self.transaction_id)
                    .ok_or_else(|| ErrorKind::TransactionNotFound {
                        transaction_id: self.transaction_id,
                    })?.ok_or_else(|| ErrorKind::TransactionNotFound {
                        transaction_id: self.transaction_id,
                    })?;

                Async::Ready(envelope)
            }
            false => Async::NotReady,
        })
    }
}
