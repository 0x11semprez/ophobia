//! Three chain nodes gossip over the Go mixnet; start `go run ./cmd/bridge` first.
use std::thread::sleep;
use std::time::{Duration, Instant};

use ophobia_chain::bridge::BridgeTransport;
use ophobia_chain::consensus::ProofOfWork;
use ophobia_chain::node::Node;
use ophobia_chain::wallet::Account;

type MixNode = Node<ProofOfWork, BridgeTransport>;

// Polls every node until all reach `height`, handing accepted blocks to the wallets.
fn settle(nodes: &mut [MixNode], height: u64, wallets: &mut [&mut Account]) {
    let deadline = Instant::now() + Duration::from_secs(120);
    while nodes.iter().any(|n| n.chain.height() < height) {
        assert!(
            Instant::now() < deadline,
            "nodes did not reach height {height}"
        );
        for node in nodes.iter_mut() {
            for block in node.poll() {
                wallets.iter_mut().for_each(|w| w.scan_block(&block));
            }
        }
        sleep(Duration::from_millis(50));
    }
}

fn main() {
    let mut addrs: Vec<String> = std::env::args().skip(1).collect();
    if addrs.is_empty() {
        addrs = (9200..9203).map(|p| format!("127.0.0.1:{p}")).collect();
    }
    let mut nodes: Vec<MixNode> = addrs
        .iter()
        .map(|a| {
            let transport = BridgeTransport::connect(a.as_str()).expect("connect to the bridge");
            let peers = transport.peers().to_vec();
            println!(
                "node {a}: id {:02x?} with {} peers",
                &transport.id()[..4],
                peers.len()
            );
            Node::new(ProofOfWork { difficulty_bits: 8 }, transport, peers)
        })
        .collect();

    let (mut alice, mut bob) = (Account::new(), Account::new());
    let started = Instant::now();
    for t in 1..=3 {
        let block = nodes[0].mine(&alice.address(), t).expect("mine");
        alice.scan_block(&block);
    }
    settle(&mut nodes, 3, &mut []);
    println!("3 blocks reached every node in {:?}", started.elapsed());

    let tx = alice
        .pay(nodes[1].chain.ledger(), &bob.address(), 60, 2, 3)
        .expect("pay");
    nodes[1].submit_tx(tx).expect("submit");
    let deadline = Instant::now() + Duration::from_secs(120);
    while nodes[0].mempool.is_empty() {
        assert!(
            Instant::now() < deadline,
            "the transaction never reached the miner"
        );
        nodes[0].poll();
        sleep(Duration::from_millis(50));
    }
    let block = nodes[0].mine(&Account::new().address(), 4).expect("mine");
    alice.scan_block(&block);
    bob.scan_block(&block);
    settle(&mut nodes, 4, &mut []);

    println!("alice {} bob {}", alice.balance(), bob.balance());
    for (i, node) in nodes.iter().enumerate() {
        println!(
            "node {i}: height {} tip {:02x?}",
            node.chain.height(),
            &node.chain.tip().hash()[..4]
        );
    }
}
