use std::collections::{HashMap, HashSet};

use crate::error::ChainError;
use crate::ledger::Ledger;
use crate::types::{Block, Hash, Tx};

/// Valid transactions waiting for a block, with no two spending the same output.
#[derive(Default)]
pub struct Mempool {
    txs: HashMap<Hash, Tx>,
    order: Vec<Hash>,
    key_images: HashSet<[u8; 32]>,
}

impl Mempool {
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Accepts `tx` if it is valid against `ledger` and conflicts with nothing pending.
    pub fn add(&mut self, ledger: &Ledger, tx: Tx) -> Result<Hash, ChainError> {
        ledger.validate_tx(&tx)?;
        if tx.inputs.iter().any(|i| self.key_images.contains(&i.key_image)) {
            return Err(ChainError::DoubleSpend);
        }
        let id = tx.id();
        if self.txs.contains_key(&id) {
            return Ok(id);
        }
        self.key_images.extend(tx.inputs.iter().map(|i| i.key_image));
        self.order.push(id);
        self.txs.insert(id, tx);
        Ok(id)
    }

    /// Up to `max` pending transactions, oldest first.
    pub fn pending(&self, max: usize) -> Vec<Tx> {
        self.order.iter().take(max).map(|id| self.txs[id].clone()).collect()
    }

    /// Drops what `block` confirmed, and anything it made unspendable.
    pub fn remove_confirmed(&mut self, block: &Block) {
        let spent: HashSet<[u8; 32]> = block.txs.iter().flat_map(|t| t.inputs.iter().map(|i| i.key_image)).collect();
        let dropped: Vec<Hash> = self
            .order
            .iter()
            .filter(|id| self.txs[*id].inputs.iter().any(|i| spent.contains(&i.key_image)))
            .copied()
            .collect();
        for id in dropped {
            if let Some(tx) = self.txs.remove(&id) {
                for input in &tx.inputs {
                    self.key_images.remove(&input.key_image);
                }
            }
            self.order.retain(|o| *o != id);
        }
    }
}
