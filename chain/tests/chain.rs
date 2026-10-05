use ophobia_chain::chain::{BLOCK_REWARD, Chain};
use ophobia_chain::consensus::{Consensus, ProofOfWork};
use ophobia_chain::error::ChainError;
use ophobia_chain::hash::merkle_root;
use ophobia_chain::mempool::Mempool;
use ophobia_chain::wallet::Account;

const POW: ProofOfWork = ProofOfWork { difficulty_bits: 8 };

/// Re-mines the header after a test edited the block.
fn reseal(block: &mut ophobia_chain::types::Block) {
    POW.seal(&mut block.header);
}

/// A chain of three blocks all mined to `alice`.
fn funded() -> (Chain<ProofOfWork>, Account) {
    let (mut chain, mut alice) = (Chain::new(POW), Account::new());
    for t in 1..=3 {
        let block = chain.mine(&alice.address(), vec![], t).unwrap();
        chain.add_block(block.clone()).unwrap();
        alice.scan_block(&block);
    }
    (chain, alice)
}

#[test]
fn mining_pays_the_reward() {
    let (chain, alice) = funded();
    assert_eq!(chain.height(), 3);
    assert_eq!(alice.balance(), 3 * BLOCK_REWARD);
}

#[test]
fn payment_goes_through_mempool_and_block() {
    let (mut chain, mut alice) = funded();
    let (mut bob, mut miner) = (Account::new(), Account::new());

    let tx = alice.pay(chain.ledger(), &bob.address(), 60, 2, 3).unwrap();
    let mut pool = Mempool::default();
    pool.add(chain.ledger(), tx).unwrap();
    assert_eq!(pool.len(), 1);

    let block = chain.mine(&miner.address(), pool.pending(10), 10).unwrap();
    chain.add_block(block.clone()).unwrap();
    pool.remove_confirmed(&block);
    for account in [&mut alice, &mut bob, &mut miner] {
        account.scan_block(&block);
    }

    assert!(pool.is_empty());
    assert_eq!(bob.balance(), 60);
    // Two 50-coin outputs spent: 60 + 2 paid, 38 change, one untouched output.
    assert_eq!(alice.balance(), 38 + BLOCK_REWARD);
    assert_eq!(miner.balance(), BLOCK_REWARD + 2);
}

#[test]
fn mempool_refuses_two_spends_of_one_output() {
    let (chain, alice) = funded();
    let bob = Account::new();
    let mut pool = Mempool::default();
    pool.add(
        chain.ledger(),
        alice.pay(chain.ledger(), &bob.address(), 10, 1, 3).unwrap(),
    )
    .unwrap();
    let again = alice.pay(chain.ledger(), &bob.address(), 11, 1, 3).unwrap();
    assert_eq!(
        pool.add(chain.ledger(), again),
        Err(ChainError::DoubleSpend)
    );
}

#[test]
fn block_with_a_double_spend_is_rejected() {
    let (chain, alice) = funded();
    let bob = Account::new();
    let a = alice.pay(chain.ledger(), &bob.address(), 10, 1, 3).unwrap();
    let b = alice.pay(chain.ledger(), &bob.address(), 11, 1, 3).unwrap();

    // `mine` silently drops the conflicting one, so a forged block must be assembled by hand.
    let mut block = chain.mine(&alice.address(), vec![a.clone()], 5).unwrap();
    block.txs.push(b);
    block.header.tx_root = merkle_root(&block.txs);
    reseal(&mut block);
    let mut chain = chain;
    assert_eq!(chain.add_block(block), Err(ChainError::DoubleSpend));
}

#[test]
fn mine_skips_candidates_that_conflict() {
    let (chain, alice) = funded();
    let bob = Account::new();
    let a = alice.pay(chain.ledger(), &bob.address(), 10, 1, 3).unwrap();
    let b = alice.pay(chain.ledger(), &bob.address(), 11, 1, 3).unwrap();
    let block = chain.mine(&alice.address(), vec![a, b], 5).unwrap();
    assert_eq!(block.txs.len(), 2); // coinbase + the first spend only
}

#[test]
fn block_that_does_not_extend_the_tip_is_rejected() {
    let (mut chain, alice) = funded();
    let mut block = chain.mine(&alice.address(), vec![], 9).unwrap();
    block.header.prev_hash = [1; 32];
    reseal(&mut block);
    assert_eq!(
        chain.add_block(block),
        Err(ChainError::BadBlock("does not extend the tip"))
    );
}

#[test]
fn tampered_transactions_break_the_tx_root() {
    let (mut chain, alice) = funded();
    let mut block = chain.mine(&alice.address(), vec![], 9).unwrap();
    block.txs[0].fee = 1;
    assert_eq!(
        chain.add_block(block),
        Err(ChainError::BadBlock("tx root mismatch"))
    );
}

#[test]
fn insufficient_work_is_rejected() {
    let alice = Account::new();
    let mut strict = Chain::new(ProofOfWork {
        difficulty_bits: 40,
    });
    // Mined at 8 bits over the strict chain's genesis: valid in every way but the work.
    let easy = Chain::new(POW);
    let mut block = easy.mine(&alice.address(), vec![], 1).unwrap();
    block.header.prev_hash = strict.tip().hash();
    reseal(&mut block);
    assert_eq!(
        strict.add_block(block),
        Err(ChainError::BadBlock("insufficient proof of work"))
    );
}

#[test]
fn inflated_coinbase_is_rejected() {
    let (mut chain, alice) = funded();
    let mut block = chain.mine(&alice.address(), vec![], 9).unwrap();
    block.txs[0] = ophobia_chain::wallet::coinbase(&alice.address(), BLOCK_REWARD + 1).unwrap();
    block.header.tx_root = merkle_root(&block.txs);
    reseal(&mut block);
    assert_eq!(
        chain.add_block(block),
        Err(ChainError::BadBlock("coinbase amount is not the reward"))
    );
}

#[test]
fn block_without_coinbase_is_rejected() {
    let (mut chain, alice) = funded();
    let mut block = chain.mine(&alice.address(), vec![], 9).unwrap();
    block.txs.clear();
    block.header.tx_root = merkle_root(&block.txs);
    reseal(&mut block);
    assert_eq!(
        chain.add_block(block),
        Err(ChainError::BadBlock("missing coinbase"))
    );
}
