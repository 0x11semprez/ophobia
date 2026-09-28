use serde::{Deserialize, Serialize};

pub type Hash = [u8; 32];

/// Confidential output: neither amount nor recipient in the clear.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    /// Pedersen commitment to the amount.
    pub commitment: [u8; 32],
    /// One-time public key (stealth address).
    pub one_time_key: [u8; 32],
    /// Ephemeral public key the recipient uses to find the output.
    pub ephemeral_key: [u8; 32],
    /// Proves 0 <= amount < 2^64.
    pub range_proof: Vec<u8>,
}

/// Spends one output of a ring without revealing which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    /// Key image: unique per spent output, blocks double spends.
    pub key_image: [u8; 32],
    /// Hashes of the outputs in the ring: decoys plus the real one.
    pub ring: Vec<Hash>,
    /// Pseudo-output commitment the ring signature proves equal to one ring member's.
    pub pseudo_commitment: [u8; 32],
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tx {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    /// Fee in the clear, committed with a zero blinding factor.
    pub fee: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub height: u64,
    pub prev_hash: Hash,
    pub tx_root: Hash,
    pub timestamp: u64,
    pub nonce: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: Header,
    pub txs: Vec<Tx>,
}
