use sha2::{Digest, Sha256};

use crate::error::ChainError;
use crate::types::{Block, Hash, Header, Output, Tx};

pub fn sha256(parts: &[&[u8]]) -> Hash {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn encode<T: serde::Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("in-memory serialization cannot fail")
}

pub fn to_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    encode(value)
}

pub fn from_bytes<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, ChainError> {
    bincode::deserialize(bytes).map_err(|e| ChainError::Encoding(e.to_string()))
}

impl Output {
    /// Identifier of the output in the ledger and in rings.
    pub fn id(&self) -> Hash {
        sha256(&[b"ophobia/output", &encode(self)])
    }
}

impl Tx {
    /// Identifier of the whole transaction, signatures included.
    pub fn id(&self) -> Hash {
        sha256(&[b"ophobia/tx", &encode(self)])
    }

    /// Hash the ring signatures commit to: the transaction with signatures blanked.
    pub fn signing_hash(&self) -> Hash {
        let mut prefix = self.clone();
        for input in &mut prefix.inputs {
            input.signature.clear();
        }
        sha256(&[b"ophobia/tx-prefix", &encode(&prefix)])
    }
}

impl Header {
    pub fn hash(&self) -> Hash {
        sha256(&[b"ophobia/header", &encode(self)])
    }
}

impl Block {
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }
}

/// Merkle root of the transaction ids; an odd node is paired with itself.
pub fn merkle_root(txs: &[Tx]) -> Hash {
    let mut level: Vec<Hash> = txs.iter().map(Tx::id).collect();
    if level.is_empty() {
        return sha256(&[b"ophobia/empty-root"]);
    }
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|pair| sha256(&[b"ophobia/node", &pair[0], pair.get(1).unwrap_or(&pair[0])]))
            .collect();
    }
    level[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(fee: u64) -> Tx {
        Tx {
            inputs: vec![],
            outputs: vec![],
            fee,
        }
    }

    #[test]
    fn merkle_root_depends_on_every_tx() {
        let a = merkle_root(&[tx(1), tx(2), tx(3)]);
        assert_ne!(a, merkle_root(&[tx(1), tx(2), tx(4)]));
        assert_ne!(a, merkle_root(&[tx(1), tx(2)]));
    }

    #[test]
    fn roundtrip() {
        let t = tx(7);
        let back: Tx = from_bytes(&to_bytes(&t)).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(from_bytes::<Tx>(&[1, 2, 3]).is_err());
    }
}
