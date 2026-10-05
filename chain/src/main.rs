//! Demo: three nodes gossip blocks and a confidential payment over an in-memory network.

use ophobia_chain::chain::BLOCK_REWARD;
use ophobia_chain::consensus::ProofOfWork;
use ophobia_chain::node::Node;
use ophobia_chain::transport::MemoryNetwork;
use ophobia_chain::wallet::Account;

fn main() {
    let net = MemoryNetwork::default();
    let ids = [[1u8; 32], [2; 32], [3; 32]];
    let mut nodes: Vec<_> = ids
        .iter()
        .map(|me| {
            let peers = ids.iter().filter(|p| *p != me).copied().collect();
            Node::new(
                ProofOfWork {
                    difficulty_bits: 12,
                },
                net.endpoint(*me),
                peers,
            )
        })
        .collect();

    let (mut alice, mut bob) = (Account::new(), Account::new());
    for t in 1..=3 {
        let block = nodes[0].mine(&alice.address(), t).expect("mine");
        alice.scan_block(&block);
        println!(
            "block {} mined, alice holds {}",
            block.header.height,
            alice.balance()
        );
    }

    for node in &mut nodes {
        node.poll();
    }

    let tx = alice
        .pay(nodes[1].chain.ledger(), &bob.address(), 60, 2, 3)
        .expect("pay");
    nodes[1].submit_tx(tx).expect("submit");
    for node in &mut nodes {
        node.poll();
    }
    let block = nodes[0].mine(&Account::new().address(), 4).expect("mine");
    alice.scan_block(&block);
    bob.scan_block(&block);
    for node in &mut nodes {
        node.poll();
    }

    println!(
        "alice {} bob {} (reward {BLOCK_REWARD})",
        alice.balance(),
        bob.balance()
    );
    for (i, node) in nodes.iter().enumerate() {
        println!(
            "node {i}: height {} tip {:02x?}",
            node.chain.height(),
            &node.chain.tip().hash()[..4]
        );
    }
}
