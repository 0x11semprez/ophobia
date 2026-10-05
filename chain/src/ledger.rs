use std::collections::{HashMap, HashSet};

use rand::seq::SliceRandom;

use crate::crypto::commitment::{balances, decompress};
use crate::crypto::rangeproof;
use crate::crypto::ring::{self, Member, Signature};
use crate::error::ChainError;
use crate::types::{Hash, Output, Tx};

pub const MIN_RING: usize = 2;
pub const MAX_RING: usize = 16;

/// Every output ever created and the key images of the spent ones.
#[derive(Default, Clone)]
pub struct Ledger {
    outputs: HashMap<Hash, Output>,
    order: Vec<Hash>,
    key_images: HashSet<[u8; 32]>,
}

impl Ledger {
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn output(&self, id: &Hash) -> Option<&Output> {
        self.outputs.get(id)
    }

    pub fn is_spent(&self, key_image: &[u8; 32]) -> bool {
        self.key_images.contains(key_image)
    }

    /// Picks up to `n` distinct outputs, never one of `exclude`, to pad a ring.
    pub fn decoys(&self, exclude: &[Hash], n: usize) -> Result<Vec<Hash>, ChainError> {
        let pool: Vec<Hash> = self
            .order
            .iter()
            .filter(|id| !exclude.contains(id))
            .copied()
            .collect();
        if pool.len() < n {
            return Err(ChainError::NotEnoughDecoys);
        }
        Ok(pool
            .choose_multiple(&mut rand::thread_rng(), n)
            .copied()
            .collect())
    }

    fn member(&self, id: &Hash) -> Result<Member, ChainError> {
        let output = self.outputs.get(id).ok_or(ChainError::UnknownRingMember)?;
        Ok(Member {
            key: decompress(&output.one_time_key)?,
            commitment: decompress(&output.commitment)?,
        })
    }

    /// Checks a spending transaction against the current state without changing it.
    pub fn validate_tx(&self, tx: &Tx) -> Result<(), ChainError> {
        if tx.inputs.is_empty() || tx.outputs.is_empty() {
            return Err(ChainError::Empty);
        }
        let message = tx.signing_hash();
        let mut images = HashSet::new();
        let mut pseudos = Vec::with_capacity(tx.inputs.len());

        for input in &tx.inputs {
            if self.key_images.contains(&input.key_image) || !images.insert(input.key_image) {
                return Err(ChainError::DoubleSpend);
            }
            if !(MIN_RING..=MAX_RING).contains(&input.ring.len())
                || input.ring.iter().collect::<HashSet<_>>().len() != input.ring.len()
            {
                return Err(ChainError::BadRing);
            }
            let members = input
                .ring
                .iter()
                .map(|id| self.member(id))
                .collect::<Result<Vec<_>, _>>()?;
            let image = decompress(&input.key_image)?;
            let pseudo = decompress(&input.pseudo_commitment)?;
            let signature = Signature::from_bytes(&input.signature)?;
            if !ring::verify(&message, &members, &image, &pseudo, &signature) {
                return Err(ChainError::BadSignature);
            }
            pseudos.push(pseudo);
        }

        let mut commitments = Vec::with_capacity(tx.outputs.len());
        for output in &tx.outputs {
            if !rangeproof::verify(&output.commitment, &output.range_proof) {
                return Err(ChainError::BadRangeProof);
            }
            commitments.push(decompress(&output.commitment)?);
        }
        if !balances(&pseudos, &commitments, tx.fee) {
            return Err(ChainError::Unbalanced);
        }
        Ok(())
    }

    /// Records a transaction already accepted by `validate_tx` (or a coinbase).
    pub fn apply_tx(&mut self, tx: &Tx) {
        for input in &tx.inputs {
            self.key_images.insert(input.key_image);
        }
        for output in &tx.outputs {
            let id = output.id();
            if self.outputs.insert(id, output.clone()).is_none() {
                self.order.push(id);
            }
        }
    }
}
