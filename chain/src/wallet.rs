//! Wallet side: finds the outputs paying us and builds signed transactions.

use std::collections::HashSet;

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;

use crate::crypto::commitment::{commit, decompress};
use crate::crypto::rangeproof;
use crate::crypto::ring::{self, Member};
use crate::crypto::stealth::{self, Address, Wallet};
use crate::error::ChainError;
use crate::ledger::{Ledger, MAX_RING};
use crate::types::{Hash, Input, Output, Tx};

/// An output we can spend.
#[derive(Clone)]
pub struct Owned {
    pub id: Hash,
    pub secret_key: Scalar,
    pub blinding: Scalar,
    pub amount: u64,
    pub key_image: [u8; 32],
}

pub struct Account {
    keys: Wallet,
    owned: Vec<Owned>,
}

/// Coinbase transaction paying `amount`, in the clear, to `to`; the blinding factor is zero so verifiers can check the sum.
pub fn coinbase(to: &Address, amount: u64) -> Result<Tx, ChainError> {
    let s = stealth::derive(to, amount, 0)?;
    let (range_proof, commitment) = rangeproof::prove(amount, &Scalar::ZERO)?;
    Ok(Tx {
        inputs: vec![],
        outputs: vec![Output {
            commitment,
            one_time_key: s.one_time_key,
            ephemeral_key: s.ephemeral_key,
            encrypted_amount: s.encrypted_amount,
            range_proof,
        }],
        fee: 0,
    })
}

impl Account {
    pub fn new() -> Self {
        Self { keys: Wallet::generate(), owned: Vec::new() }
    }

    pub fn address(&self) -> Address {
        self.keys.address()
    }

    pub fn balance(&self) -> u64 {
        self.owned.iter().map(|o| o.amount).sum()
    }

    /// Records the outputs of `tx` that pay us and forgets the ones its inputs spend.
    pub fn scan_tx(&mut self, tx: &Tx) {
        let spent: HashSet<[u8; 32]> = tx.inputs.iter().map(|i| i.key_image).collect();
        self.owned.retain(|o| !spent.contains(&o.key_image));

        for (index, output) in tx.outputs.iter().enumerate() {
            let Some(got) = self.keys.scan(&output.one_time_key, &output.ephemeral_key, &output.encrypted_amount, index as u64)
            else {
                continue;
            };
            // The commitment, not the sender's word, decides what the output is worth.
            let Ok(committed) = decompress(&output.commitment) else { continue };
            let blinding = [got.blinding, Scalar::ZERO].into_iter().find(|b| commit(got.amount, b) == committed);
            let Some(blinding) = blinding else { continue };
            self.owned.push(Owned {
                id: output.id(),
                secret_key: got.secret_key,
                blinding,
                amount: got.amount,
                key_image: ring::key_image(&got.secret_key).compress().to_bytes(),
            });
        }
    }

    /// Builds a transaction paying `amount` to `to`, sending any change back to this account.
    ///
    /// Spent outputs stay in the balance until a transaction spending them is scanned.
    pub fn pay(&self, ledger: &Ledger, to: &Address, amount: u64, fee: u64, ring_size: usize) -> Result<Tx, ChainError> {
        if !(2..=MAX_RING).contains(&ring_size) {
            return Err(ChainError::BadRing);
        }
        let needed = amount.checked_add(fee).ok_or(ChainError::InsufficientFunds)?;
        let mut picked = Vec::new();
        let mut total = 0u64;
        for output in &self.owned {
            if total >= needed {
                break;
            }
            total += output.amount;
            picked.push(output);
        }
        if total < needed {
            return Err(ChainError::InsufficientFunds);
        }

        let mut payments = vec![(*to, amount)];
        if total > needed {
            payments.push((self.address(), total - needed));
        }
        let mut outputs = Vec::new();
        let mut out_blinding = Scalar::ZERO;
        for (index, (address, value)) in payments.iter().enumerate() {
            let s = stealth::derive(address, *value, index as u64)?;
            let (range_proof, commitment) = rangeproof::prove(*value, &s.blinding)?;
            out_blinding += s.blinding;
            outputs.push(Output {
                commitment,
                one_time_key: s.one_time_key,
                ephemeral_key: s.ephemeral_key,
                encrypted_amount: s.encrypted_amount,
                range_proof,
            });
        }

        // Pseudo blindings must sum to the output blindings for the amounts to balance.
        let mut pseudo_blindings: Vec<Scalar> = (1..picked.len()).map(|_| Scalar::random(&mut OsRng)).collect();
        pseudo_blindings.push(out_blinding - pseudo_blindings.iter().sum::<Scalar>());

        let mut rings = Vec::new();
        let mut inputs = Vec::new();
        for (spend, pseudo_blinding) in picked.iter().zip(&pseudo_blindings) {
            let mut ids = ledger.decoys(&[spend.id], ring_size - 1)?;
            ids.push(spend.id);
            ids.shuffle(&mut OsRng);
            let real = ids.iter().position(|id| *id == spend.id).expect("real output was just pushed");
            rings.push((ids, real));
            inputs.push(Input {
                key_image: spend.key_image,
                ring: rings.last().unwrap().0.clone(),
                pseudo_commitment: commit(spend.amount, pseudo_blinding).compress().to_bytes(),
                signature: Vec::new(),
            });
        }

        let mut tx = Tx { inputs, outputs, fee };
        let message = tx.signing_hash();
        for (i, spend) in picked.iter().enumerate() {
            let (ids, real) = &rings[i];
            let members = ids
                .iter()
                .map(|id| {
                    let o = ledger.output(id).ok_or(ChainError::UnknownRingMember)?;
                    Ok(Member { key: decompress(&o.one_time_key)?, commitment: decompress(&o.commitment)? })
                })
                .collect::<Result<Vec<_>, ChainError>>()?;
            debug_assert_eq!(members[*real].key, spend.secret_key * G);
            let pseudo = decompress(&tx.inputs[i].pseudo_commitment)?;
            let z = spend.blinding - pseudo_blindings[i];
            tx.inputs[i].signature = ring::sign(&message, &members, *real, &spend.secret_key, &z, &pseudo).to_bytes();
        }
        Ok(tx)
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new()
    }
}
