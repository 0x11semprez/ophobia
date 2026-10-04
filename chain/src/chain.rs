use crate::consensus::Consensus;
use crate::crypto::commitment::{commit, decompress};
use crate::crypto::rangeproof;
use crate::crypto::stealth::Address;
use crate::error::ChainError;
use crate::hash::merkle_root;
use crate::ledger::Ledger;
use crate::types::{Block, Header, Tx};
use crate::wallet::coinbase;
use curve25519_dalek::scalar::Scalar;

/// Amount created by each block, on top of the fees it collects.
pub const BLOCK_REWARD: u64 = 50;
/// Spending transactions per block, coinbase excluded.
pub const MAX_BLOCK_TXS: usize = 128;

/// Linear chain: one tip, no reorganisation.
pub struct Chain<C: Consensus> {
    blocks: Vec<Block>,
    ledger: Ledger,
    consensus: C,
}

fn genesis() -> Block {
    Block {
        header: Header { height: 0, prev_hash: [0; 32], tx_root: merkle_root(&[]), timestamp: 0, nonce: 0 },
        txs: Vec::new(),
    }
}

impl<C: Consensus> Chain<C> {
    pub fn new(consensus: C) -> Self {
        Self { blocks: vec![genesis()], ledger: Ledger::default(), consensus }
    }

    pub fn tip(&self) -> &Block {
        self.blocks.last().expect("chain always holds the genesis block")
    }

    pub fn height(&self) -> u64 {
        self.tip().header.height
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Checks `block` as the next block and returns the ledger it produces.
    fn check(&self, block: &Block) -> Result<Ledger, ChainError> {
        let tip = self.tip();
        let header = &block.header;
        if header.height != tip.header.height + 1 {
            return Err(ChainError::BadBlock("wrong height"));
        }
        if header.prev_hash != tip.hash() {
            return Err(ChainError::BadBlock("does not extend the tip"));
        }
        if header.timestamp < tip.header.timestamp {
            return Err(ChainError::BadBlock("timestamp goes backwards"));
        }
        if header.tx_root != merkle_root(&block.txs) {
            return Err(ChainError::BadBlock("tx root mismatch"));
        }
        self.consensus.verify(header)?;

        let (base, spends) = block.txs.split_first().ok_or(ChainError::BadBlock("missing coinbase"))?;
        if spends.len() > MAX_BLOCK_TXS {
            return Err(ChainError::BadBlock("too many transactions"));
        }

        let mut ledger = self.ledger.clone();
        let mut fees = 0u64;
        for tx in spends {
            ledger.validate_tx(tx)?;
            fees = fees.checked_add(tx.fee).ok_or(ChainError::BadBlock("fee overflow"))?;
            ledger.apply_tx(tx);
        }
        let reward = BLOCK_REWARD.checked_add(fees).ok_or(ChainError::BadBlock("fee overflow"))?;
        check_coinbase(base, reward)?;
        ledger.apply_tx(base);
        Ok(ledger)
    }

    pub fn add_block(&mut self, block: Block) -> Result<(), ChainError> {
        self.ledger = self.check(&block)?;
        self.blocks.push(block);
        Ok(())
    }

    /// Mines the next block paying `miner`, keeping the candidates that still validate in order.
    pub fn mine(&self, miner: &Address, candidates: Vec<Tx>, timestamp: u64) -> Result<Block, ChainError> {
        let mut scratch = self.ledger.clone();
        let mut spends = Vec::new();
        let mut fees = 0u64;
        for tx in candidates.into_iter().take(MAX_BLOCK_TXS) {
            if scratch.validate_tx(&tx).is_ok() {
                fees += tx.fee;
                scratch.apply_tx(&tx);
                spends.push(tx);
            }
        }

        let mut txs = vec![coinbase(miner, BLOCK_REWARD + fees)?];
        txs.extend(spends);
        let tip = self.tip();
        let mut header = Header {
            height: tip.header.height + 1,
            prev_hash: tip.hash(),
            tx_root: merkle_root(&txs),
            timestamp: timestamp.max(tip.header.timestamp),
            nonce: 0,
        };
        self.consensus.seal(&mut header);
        Ok(Block { header, txs })
    }
}

/// A coinbase has no inputs and one output whose commitment is exactly `reward`, unblinded.
fn check_coinbase(tx: &Tx, reward: u64) -> Result<(), ChainError> {
    if !tx.inputs.is_empty() || tx.outputs.len() != 1 || tx.fee != 0 {
        return Err(ChainError::BadBlock("malformed coinbase"));
    }
    let output = &tx.outputs[0];
    if decompress(&output.commitment)? != commit(reward, &Scalar::ZERO) {
        return Err(ChainError::BadBlock("coinbase amount is not the reward"));
    }
    if !rangeproof::verify(&output.commitment, &output.range_proof) {
        return Err(ChainError::BadRangeProof);
    }
    Ok(())
}
