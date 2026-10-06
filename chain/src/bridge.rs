//! Transport over the local TCP bridge to the Go mixnet.
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::error::ChainError;
use crate::message::MAX_FRAME;
use crate::transport::{PeerId, Transport};

// Every message is a type byte, a big-endian u16 length and the payload.
const HELLO: u8 = 0;
const SEND: u8 = 1;
const FRAME: u8 = 2;
const PEERS: u8 = 3;
const ID_SIZE: usize = 32;

type Inbox = Arc<Mutex<VecDeque<Vec<u8>>>>;

fn transport_error(e: io::Error) -> ChainError {
    ChainError::Transport(e.to_string())
}

fn read_message(reader: &mut impl Read) -> io::Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 3];
    reader.read_exact(&mut header)?;
    let mut payload = vec![0u8; u16::from_be_bytes([header[1], header[2]]) as usize];
    reader.read_exact(&mut payload)?;
    Ok((header[0], payload))
}

fn write_message(writer: &mut impl Write, kind: u8, payload: &[u8]) -> io::Result<()> {
    let len = u16::try_from(payload.len()).map_err(|_| io::Error::other("payload too large"))?;
    let mut buf = Vec::with_capacity(3 + payload.len());
    buf.push(kind);
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(payload);
    writer.write_all(&buf)
}

/// One node's connection to its mixnet client; frames travel as mixnet messages.
pub struct BridgeTransport {
    id: PeerId,
    peers: Vec<PeerId>,
    writer: Mutex<TcpStream>,
    inbox: Inbox,
}

impl BridgeTransport {
    /// Connects, learns this client's id and the other clients, then receives in the background.
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self, ChainError> {
        let stream = TcpStream::connect(addr).map_err(transport_error)?;
        let mut reader = stream.try_clone().map_err(transport_error)?;
        let mut writer = stream;

        let (kind, payload) = read_message(&mut reader).map_err(transport_error)?;
        let id: PeerId = match (kind, payload.try_into()) {
            (HELLO, Ok(id)) => id,
            _ => {
                return Err(ChainError::Transport(
                    "bridge did not greet with an id".into(),
                ));
            }
        };

        write_message(&mut writer, PEERS, &[]).map_err(transport_error)?;
        let inbox: Inbox = Arc::default();
        let peers = loop {
            let (kind, payload) = read_message(&mut reader).map_err(transport_error)?;
            match kind {
                PEERS if payload.len() % ID_SIZE == 0 => {
                    break payload
                        .chunks(ID_SIZE)
                        .map(|c| c.try_into().unwrap())
                        .collect();
                }
                PEERS => return Err(ChainError::Transport("malformed peer list".into())),
                // A frame can arrive before the peer list; keep it.
                FRAME => inbox.lock().unwrap().push_back(payload),
                _ => {}
            }
        };

        let background = inbox.clone();
        thread::spawn(move || {
            while let Ok((kind, payload)) = read_message(&mut reader) {
                if kind == FRAME {
                    background.lock().unwrap().push_back(payload);
                }
            }
        });
        Ok(Self {
            id,
            peers,
            writer: Mutex::new(writer),
            inbox,
        })
    }

    /// This node's own id on the mixnet.
    pub fn id(&self) -> PeerId {
        self.id
    }

    /// Every other node reachable through the bridge.
    pub fn peers(&self) -> &[PeerId] {
        &self.peers
    }
}

impl Transport for BridgeTransport {
    fn send(&self, to: &PeerId, frame: &[u8]) -> Result<(), ChainError> {
        if frame.len() > MAX_FRAME {
            return Err(ChainError::Transport(format!(
                "frame of {} bytes exceeds {MAX_FRAME}",
                frame.len()
            )));
        }
        let mut payload = to.to_vec();
        payload.extend_from_slice(frame);
        write_message(&mut *self.writer.lock().unwrap(), SEND, &payload).map_err(transport_error)
    }

    fn recv(&self) -> Vec<Vec<u8>> {
        self.inbox.lock().unwrap().drain(..).collect()
    }
}

impl Drop for BridgeTransport {
    fn drop(&mut self) {
        let _ = self.writer.lock().unwrap().shutdown(Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    const ME: PeerId = [7; 32];
    const BOB: PeerId = [8; 32];
    const EVE: PeerId = [9; 32];

    // Plays the bridge side: greets, answers the peer request, pushes one frame early, reports the first send.
    fn fake_bridge(early_frame: bool) -> (String, mpsc::Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            write_message(&mut conn, HELLO, &ME).unwrap();
            assert_eq!(read_message(&mut conn).unwrap(), (PEERS, vec![]));
            if early_frame {
                write_message(&mut conn, FRAME, b"early").unwrap();
            }
            write_message(&mut conn, PEERS, &[BOB, EVE].concat()).unwrap();
            write_message(&mut conn, FRAME, b"hello").unwrap();
            while let Ok((kind, payload)) = read_message(&mut conn) {
                if kind == SEND {
                    tx.send(payload).unwrap();
                }
            }
        });
        (addr, rx)
    }

    fn wait_for(transport: &BridgeTransport, count: usize) -> Vec<Vec<u8>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut got = Vec::new();
        while got.len() < count && Instant::now() < deadline {
            got.extend(transport.recv());
            thread::sleep(Duration::from_millis(10));
        }
        got
    }

    #[test]
    fn handshake_gives_id_and_peers() {
        let (addr, _rx) = fake_bridge(false);
        let t = BridgeTransport::connect(addr).unwrap();
        assert_eq!(t.id(), ME);
        assert_eq!(t.peers(), &[BOB, EVE]);
    }

    #[test]
    fn frames_are_received_in_order_even_if_early() {
        let (addr, _rx) = fake_bridge(true);
        let t = BridgeTransport::connect(addr).unwrap();
        assert_eq!(wait_for(&t, 2), vec![b"early".to_vec(), b"hello".to_vec()]);
    }

    #[test]
    fn send_addresses_the_peer() {
        let (addr, rx) = fake_bridge(false);
        let t = BridgeTransport::connect(addr).unwrap();
        t.send(&BOB, b"payload").unwrap();
        let got = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(&got[..32], &BOB);
        assert_eq!(&got[32..], b"payload");
    }

    #[test]
    fn oversized_frame_is_refused() {
        let (addr, _rx) = fake_bridge(false);
        let t = BridgeTransport::connect(addr).unwrap();
        assert!(t.send(&BOB, &[0; MAX_FRAME + 1]).is_err());
    }

    #[test]
    fn bad_greeting_is_an_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            write_message(&mut conn, FRAME, b"nope").unwrap();
        });
        assert!(BridgeTransport::connect(addr).is_err());
    }

    #[test]
    fn nothing_listening_is_an_error() {
        let addr = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        assert!(BridgeTransport::connect(addr).is_err());
    }
}
