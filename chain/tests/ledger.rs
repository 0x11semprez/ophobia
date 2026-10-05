use ophobia_chain::error::ChainError;
use ophobia_chain::ledger::Ledger;
use ophobia_chain::wallet::{Account, coinbase};

/// A ledger where `alice` owns three coinbase outputs of 100 and `bob` one of 100.
fn setup() -> (Ledger, Account, Account) {
    let (mut ledger, mut alice, mut bob) = (Ledger::default(), Account::new(), Account::new());
    for _ in 0..3 {
        let tx = coinbase(&alice.address(), 100).unwrap();
        ledger.apply_tx(&tx);
        alice.scan_tx(&tx);
    }
    let tx = coinbase(&bob.address(), 100).unwrap();
    ledger.apply_tx(&tx);
    bob.scan_tx(&tx);
    (ledger, alice, bob)
}

#[test]
fn payment_validates_and_moves_funds() {
    let (mut ledger, mut alice, mut bob) = setup();
    assert_eq!(alice.balance(), 300);

    let tx = alice.pay(&ledger, &bob.address(), 130, 5, 4).unwrap();
    ledger.validate_tx(&tx).unwrap();
    ledger.apply_tx(&tx);
    alice.scan_tx(&tx);
    bob.scan_tx(&tx);

    assert_eq!(bob.balance(), 230);
    // Two outputs (200) were spent: 135 paid out, 65 back as change, plus the untouched third output.
    assert_eq!(alice.balance(), 65 + 100);
}

#[test]
fn spending_the_same_output_twice_is_rejected() {
    let (mut ledger, alice, bob) = setup();
    let tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    ledger.validate_tx(&tx).unwrap();
    ledger.apply_tx(&tx);
    assert_eq!(ledger.validate_tx(&tx), Err(ChainError::DoubleSpend));
}

#[test]
fn tampered_fee_breaks_the_signature() {
    let (ledger, alice, bob) = setup();
    let mut tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    tx.fee += 1;
    assert_eq!(ledger.validate_tx(&tx), Err(ChainError::BadSignature));
}

#[test]
fn tampered_output_breaks_the_signature() {
    let (ledger, alice, bob) = setup();
    let mut tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    tx.outputs[0].one_time_key = tx.outputs[1].one_time_key;
    assert_eq!(ledger.validate_tx(&tx), Err(ChainError::BadSignature));
}

#[test]
fn tampered_range_proof_is_rejected() {
    let (ledger, alice, bob) = setup();
    let mut tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    // The proof is covered by the signature, so either check may fire first; both must reject.
    let last = tx.outputs[0].range_proof.len() - 1;
    tx.outputs[0].range_proof[last] ^= 1;
    assert!(matches!(
        ledger.validate_tx(&tx),
        Err(ChainError::BadSignature) | Err(ChainError::BadRangeProof)
    ));
}

#[test]
fn unknown_ring_member_is_rejected() {
    let (ledger, alice, bob) = setup();
    let mut tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    tx.inputs[0].ring[0] = [9; 32];
    assert_eq!(ledger.validate_tx(&tx), Err(ChainError::UnknownRingMember));
}

#[test]
fn duplicate_ring_member_is_rejected() {
    let (ledger, alice, bob) = setup();
    let mut tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    tx.inputs[0].ring[1] = tx.inputs[0].ring[0];
    assert_eq!(ledger.validate_tx(&tx), Err(ChainError::BadRing));
}

#[test]
fn cannot_spend_more_than_owned() {
    let (ledger, alice, bob) = setup();
    assert_eq!(
        alice.pay(&ledger, &bob.address(), 301, 0, 3).err(),
        Some(ChainError::InsufficientFunds)
    );
}

#[test]
fn ring_needs_enough_decoys() {
    let (ledger, alice, bob) = setup();
    assert_eq!(
        alice.pay(&ledger, &bob.address(), 10, 1, 16).err(),
        Some(ChainError::NotEnoughDecoys)
    );
}

#[test]
fn outsiders_cannot_read_amounts() {
    let (ledger, alice, bob) = setup();
    let tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    let mut eve = Account::new();
    eve.scan_tx(&tx);
    assert_eq!(eve.balance(), 0);
}

#[test]
fn lying_about_the_amount_is_ignored_by_the_recipient() {
    let (ledger, alice, mut bob) = setup();
    let mut tx = alice.pay(&ledger, &bob.address(), 10, 1, 3).unwrap();
    tx.outputs[0].encrypted_amount[0] ^= 0xff;
    let before = bob.balance();
    bob.scan_tx(&tx);
    assert_eq!(bob.balance(), before);
}
