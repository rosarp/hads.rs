use std::{collections::HashMap, io, net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpStream,
    sync::{mpsc, Mutex},
};
use tokio_util::codec::{Framed, LinesCodec};

type Tx = mpsc::UnboundedSender<String>;
type Rx = mpsc::UnboundedReceiver<String>;

/// Data that is shared between all peers in the chat server.
///
/// This is the set of `Tx` handles for all connected clients. Whenever a
/// message is received from a client, it is broadcasted to all peers by
/// iterating over the `peers` entries and sending a copy of the message on each
/// `Tx`.
pub struct Shared {
    pub peers: HashMap<SocketAddr, Tx>,
}

/// The state for each connected client.
pub struct Account {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    pub lines: Framed<TcpStream, LinesCodec>,

    /// Receive half of the message channel.
    ///
    /// This is used to receive messages from peers. When a message is received
    /// off of this `Rx`, it will be written to the socket.
    pub rx: Rx,
}

impl Shared {
    /// Create a new, empty, instance of `Shared`.
    pub fn new() -> Self {
        Shared {
            peers: HashMap::new(),
        }
    }

    /// Send a `LineCodec` encoded message to every peer, except
    /// for the sender.
    pub async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        }
    }
}

impl Account {
    /// Create a new instance of `Account`.
    pub async fn new(
        state: Arc<Mutex<Shared>>,
        lines: Framed<TcpStream, LinesCodec>,
    ) -> io::Result<Account> {
        // Get the client socket address
        let addr = lines.get_ref().peer_addr()?;

        // Create a channel for this account
        let (tx, rx) = mpsc::unbounded_channel();

        // Add an entry for this `Account` in the shared state map.
        state.lock().await.peers.insert(addr, tx);

        Ok(Account { lines, rx })
    }
}
