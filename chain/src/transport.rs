use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::error::ChainError;
use crate::message::MAX_FRAME;

/// Same size as the mixnet's NodeID: an x25519 public key.
pub type PeerId = [u8; 32];

/// Carries frames between nodes. The mixnet implements it; `MemoryNetwork` stands in for tests.
pub trait Transport {
    fn send(&self, to: &PeerId, frame: &[u8]) -> Result<(), ChainError>;
    /// Frames received since the last call.
    fn recv(&self) -> Vec<Vec<u8>>;
}

type Queues = Arc<Mutex<HashMap<PeerId, VecDeque<Vec<u8>>>>>;

/// In-process network: every endpoint has an inbox, frames to unknown peers vanish like on a mixnet.
#[derive(Clone, Default)]
pub struct MemoryNetwork {
    queues: Queues,
}

pub struct MemoryTransport {
    me: PeerId,
    queues: Queues,
}

impl MemoryNetwork {
    pub fn endpoint(&self, me: PeerId) -> MemoryTransport {
        self.queues.lock().unwrap().entry(me).or_default();
        MemoryTransport {
            me,
            queues: self.queues.clone(),
        }
    }
}

impl Transport for MemoryTransport {
    fn send(&self, to: &PeerId, frame: &[u8]) -> Result<(), ChainError> {
        if frame.len() > MAX_FRAME {
            return Err(ChainError::Encoding(format!(
                "frame of {} bytes exceeds {MAX_FRAME}",
                frame.len()
            )));
        }
        if let Some(queue) = self.queues.lock().unwrap().get_mut(to) {
            queue.push_back(frame.to_vec());
        }
        Ok(())
    }

    fn recv(&self) -> Vec<Vec<u8>> {
        let mut queues = self.queues.lock().unwrap();
        queues
            .get_mut(&self.me)
            .map(|q| q.drain(..).collect())
            .unwrap_or_default()
    }
}
