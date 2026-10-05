use ophobia_chain::chain::BLOCK_REWARD;
use ophobia_chain::consensus::ProofOfWork;
use ophobia_chain::node::Node;
use ophobia_chain::transport::{MemoryNetwork, MemoryTransport, PeerId};
use ophobia_chain::wallet::Account;

const POW: ProofOfWork = ProofOfWork { difficulty_bits: 8 };

type TestNode = Node<ProofOfWork, MemoryTransport>;

/// Three nodes, each knowing the other two.
fn mesh() -> Vec<TestNode> {
    let net = MemoryNetwork::default();
    let ids: Vec<PeerId> = (1..=3u8).map(|i| [i; 32]).collect();
    ids.iter()
        .map(|me| {
            let peers = ids.iter().filter(|p| *p != me).copied().collect();
            Node::new(POW, net.endpoint(*me), peers)
        })
        .collect()
}

/// Lets gossip run for a few rounds, handing every accepted block to `wallets`.
fn settle(nodes: &mut [TestNode], wallets: &mut [&mut Account]) {
    for _ in 0..4 {
        for node in nodes.iter_mut() {
            for block in node.poll() {
                for wallet in wallets.iter_mut() {
                    wallet.scan_block(&block);
                }
            }
        }
    }
}

#[test]
fn nodes_converge_on_mined_blocks_and_payments() {
    let mut nodes = mesh();
    let (mut alice, mut bob) = (Account::new(), Account::new());

    // Node 0 mines three blocks to alice. Its own blocks are scanned directly.
    for t in 1..=3 {
        let block = nodes[0].mine(&alice.address(), t).unwrap();
        alice.scan_block(&block);
    }
    settle(&mut nodes, &mut []);
    assert!(nodes.iter().all(|n| n.chain.height() == 3));
    assert_eq!(alice.balance(), 3 * BLOCK_REWARD);

    // Alice pays bob through node 2; the transaction crosses the network to the miner.
    let tx = alice
        .pay(nodes[2].chain.ledger(), &bob.address(), 60, 2, 3)
        .unwrap();
    nodes[2].submit_tx(tx).unwrap();
    settle(&mut nodes, &mut []);
    assert!(nodes.iter().all(|n| n.mempool.len() == 1));

    let miner = Account::new();
    let block = nodes[0].mine(&miner.address(), 4).unwrap();
    for wallet in [&mut alice, &mut bob] {
        wallet.scan_block(&block);
    }
    settle(&mut nodes, &mut []);

    let tips: Vec<_> = nodes.iter().map(|n| n.chain.tip().hash()).collect();
    assert!(
        tips.windows(2).all(|w| w[0] == w[1]),
        "nodes disagree on the tip"
    );
    assert!(
        nodes
            .iter()
            .all(|n| n.chain.height() == 4 && n.mempool.is_empty())
    );
    assert_eq!(bob.balance(), 60);
    assert_eq!(alice.balance(), 38 + BLOCK_REWARD);
}

#[test]
fn garbage_frames_do_not_disturb_a_node() {
    let net = MemoryNetwork::default();
    let attacker = net.endpoint([9; 32]);
    let mut node: TestNode = Node::new(POW, net.endpoint([1; 32]), vec![]);
    use ophobia_chain::transport::Transport;
    for junk in [vec![], vec![0; 4], vec![0xff; 1021], vec![1; 13]] {
        attacker.send(&[1; 32], &junk).unwrap();
    }
    assert!(node.poll().is_empty());
    assert_eq!(node.chain.height(), 0);
}
