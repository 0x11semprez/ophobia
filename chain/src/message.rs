//! Network messages, cut into frames that fit one mixnet payload and put back together.

use std::collections::{HashMap, VecDeque};

use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::error::ChainError;
use crate::types::{Block, Tx};

/// Largest frame the mixnet carries: models.MaxMessageSize, PayloadSize (1024) minus the 3-byte header.
pub const MAX_FRAME: usize = 1021;
const ID_SIZE: usize = 8;
const HEADER: usize = ID_SIZE + 2 + 2;
const CHUNK: usize = MAX_FRAME - HEADER;
/// Fragments per message: bounds what one sender can make a receiver buffer.
pub const MAX_FRAGMENTS: usize = 1024;
/// Messages being reassembled at once; the oldest is dropped past this.
pub const MAX_PENDING: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Tx(Tx),
    Block(Block),
}

/// Cuts `message` into frames of at most `MAX_FRAME` bytes.
pub fn fragment(message: &[u8]) -> Result<Vec<Vec<u8>>, ChainError> {
    let chunks: Vec<&[u8]> = if message.is_empty() {
        vec![&[]]
    } else {
        message.chunks(CHUNK).collect()
    };
    if chunks.len() > MAX_FRAGMENTS {
        return Err(ChainError::Encoding(format!(
            "message needs {} fragments, limit is {MAX_FRAGMENTS}",
            chunks.len()
        )));
    }
    let mut id = [0u8; ID_SIZE];
    OsRng.fill_bytes(&mut id);
    let total = (chunks.len() as u16).to_be_bytes();
    Ok(chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut frame = Vec::with_capacity(HEADER + chunk.len());
            frame.extend_from_slice(&id);
            frame.extend_from_slice(&(i as u16).to_be_bytes());
            frame.extend_from_slice(&total);
            frame.extend_from_slice(chunk);
            frame
        })
        .collect())
}

struct Partial {
    total: usize,
    parts: Vec<Option<Vec<u8>>>,
    received: usize,
}

/// Collects frames, in any order and with duplicates, until a message is whole.
#[derive(Default)]
pub struct Reassembler {
    partial: HashMap<[u8; ID_SIZE], Partial>,
    order: VecDeque<[u8; ID_SIZE]>,
}

impl Reassembler {
    /// Returns the whole message once its last fragment arrives; malformed frames are dropped.
    pub fn push(&mut self, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() < HEADER || frame.len() > MAX_FRAME {
            return None;
        }
        let id: [u8; ID_SIZE] = frame[..ID_SIZE].try_into().ok()?;
        let index = u16::from_be_bytes([frame[ID_SIZE], frame[ID_SIZE + 1]]) as usize;
        let total = u16::from_be_bytes([frame[ID_SIZE + 2], frame[ID_SIZE + 3]]) as usize;
        if total == 0 || total > MAX_FRAGMENTS || index >= total {
            return None;
        }

        if !self.partial.contains_key(&id) {
            if self.order.len() >= MAX_PENDING
                && let Some(oldest) = self.order.pop_front()
            {
                self.partial.remove(&oldest);
            }
            self.order.push_back(id);
            self.partial.insert(
                id,
                Partial {
                    total,
                    parts: vec![None; total],
                    received: 0,
                },
            );
        }
        let partial = self.partial.get_mut(&id)?;
        if partial.total != total {
            return None;
        }
        if partial.parts[index].is_none() {
            partial.parts[index] = Some(frame[HEADER..].to_vec());
            partial.received += 1;
        }
        if partial.received < partial.total {
            return None;
        }

        let done = self.partial.remove(&id)?;
        self.order.retain(|o| *o != id);
        Some(done.parts.into_iter().flatten().flatten().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i % 251) as u8).collect()
    }

    fn reassemble(frames: Vec<Vec<u8>>) -> Option<Vec<u8>> {
        let mut r = Reassembler::default();
        frames.iter().fold(None, |_, f| r.push(f))
    }

    #[test]
    fn frames_fit_the_mixnet_payload() {
        let frames = fragment(&payload(10_000)).unwrap();
        assert!(frames.iter().all(|f| f.len() <= MAX_FRAME));
        assert_eq!(frames.len(), 10_000_usize.div_ceil(CHUNK));
    }

    #[test]
    fn roundtrip_for_every_size_around_a_boundary() {
        for n in [0, 1, CHUNK - 1, CHUNK, CHUNK + 1, 3 * CHUNK, 3 * CHUNK + 1] {
            let message = payload(n);
            assert_eq!(
                reassemble(fragment(&message).unwrap()),
                Some(message),
                "size {n}"
            );
        }
    }

    #[test]
    fn order_and_duplicates_do_not_matter() {
        let message = payload(5 * CHUNK);
        let mut frames = fragment(&message).unwrap();
        frames.reverse();
        frames.push(frames[0].clone());
        let mut r = Reassembler::default();
        let out: Vec<_> = frames.iter().filter_map(|f| r.push(f)).collect();
        assert_eq!(out, vec![message]);
    }

    #[test]
    fn two_messages_interleave() {
        let (a, b) = (payload(2 * CHUNK), payload(3 * CHUNK + 5));
        let (fa, fb) = (fragment(&a).unwrap(), fragment(&b).unwrap());
        let mut r = Reassembler::default();
        let mut got = Vec::new();
        for (i, frame) in fb.iter().enumerate() {
            if let Some(f) = fa.get(i) {
                got.extend(r.push(f));
            }
            got.extend(r.push(frame));
        }
        got.sort_by_key(Vec::len);
        assert_eq!(got, vec![a, b]);
    }

    #[test]
    fn malformed_frames_are_dropped() {
        let mut r = Reassembler::default();
        assert!(r.push(&[1, 2, 3]).is_none());
        assert!(r.push(&[0; MAX_FRAME + 1]).is_none());
        let mut zero_total = vec![0u8; HEADER];
        zero_total[ID_SIZE + 3] = 0;
        assert!(r.push(&zero_total).is_none());
        let mut bad_index = vec![0u8; HEADER];
        bad_index[ID_SIZE + 3] = 2; // total = 2
        bad_index[ID_SIZE + 1] = 2; // index = 2
        assert!(r.push(&bad_index).is_none());
    }

    #[test]
    fn oversized_messages_are_refused() {
        assert!(fragment(&payload(MAX_FRAGMENTS * CHUNK + 1)).is_err());
    }

    #[test]
    fn pending_messages_are_bounded() {
        let mut r = Reassembler::default();
        for _ in 0..(MAX_PENDING * 2) {
            // First fragment of a two-fragment message that never completes.
            r.push(&fragment(&payload(2 * CHUNK)).unwrap()[0]);
        }
        assert!(r.partial.len() <= MAX_PENDING);
    }
}
