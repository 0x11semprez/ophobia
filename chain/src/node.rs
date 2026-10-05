use crate::chain::Chain;
use crate::consensus::Consensus;
use crate::crypto::stealth::Address;
use crate::error::ChainError;
use crate::hash::{from_bytes, to_bytes};
use crate::mempool::Mempool;
use crate::message::{Message, Reassembler, fragment};
use crate::transport::{PeerId, Transport};
use crate::types::{Block, Tx};

/// A full node: keeps the chain and mempool and gossips transactions and blocks to its peers.
pub struct Node<C: Consensus, T: Transport> {
    pub chain: Chain<C>,
    pub mempool: Mempool,
    transport: T,
    peers: Vec<PeerId>,
    reassembler: Reassembler,
}

impl<C: Consensus, T: Transport> Node<C, T> {
    pub fn new(consensus: C, transport: T, peers: Vec<PeerId>) -> Self {
        Self {
            chain: Chain::new(consensus),
            mempool: Mempool::default(),
            transport,
            peers,
            reassembler: Reassembler::default(),
        }
    }

    fn broadcast(&self, message: &Message) -> Result<(), ChainError> {
        for frame in fragment(&to_bytes(message))? {
            for peer in &self.peers {
                self.transport.send(peer, &frame)?;
            }
        }
        Ok(())
    }

    /// Accepts a locally built transaction and gossips it.
    pub fn submit_tx(&mut self, tx: Tx) -> Result<(), ChainError> {
        self.mempool.add(self.chain.ledger(), tx.clone())?;
        self.broadcast(&Message::Tx(tx))
    }

    /// Mines a block from the mempool paying `miner`, adds it and gossips it.
    pub fn mine(&mut self, miner: &Address, timestamp: u64) -> Result<Block, ChainError> {
        let block = self.chain.mine(
            miner,
            self.mempool.pending(crate::chain::MAX_BLOCK_TXS),
            timestamp,
        )?;
        self.chain.add_block(block.clone())?;
        self.mempool.remove_confirmed(&block);
        self.broadcast(&Message::Block(block.clone()))?;
        Ok(block)
    }

    /// Processes what arrived; returns the blocks that extended the chain, for wallets to scan.
    pub fn poll(&mut self) -> Vec<Block> {
        let mut accepted = Vec::new();
        for frame in self.transport.recv() {
            let Some(bytes) = self.reassembler.push(&frame) else {
                continue;
            };
            let Ok(message) = from_bytes::<Message>(&bytes) else {
                continue;
            };
            match message {
                Message::Tx(tx) => {
                    if self.mempool.contains(&tx.id()) {
                        continue;
                    }
                    if self.mempool.add(self.chain.ledger(), tx.clone()).is_ok() {
                        let _ = self.broadcast(&Message::Tx(tx));
                    }
                }
                Message::Block(block) => {
                    if self.chain.add_block(block.clone()).is_ok() {
                        self.mempool.remove_confirmed(&block);
                        let _ = self.broadcast(&Message::Block(block.clone()));
                        accepted.push(block);
                    }
                }
            }
        }
        accepted
    }
}
