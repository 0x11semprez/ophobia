//! Pedersen commitments on Ristretto: C = v*B + r*B_blinding.

use bulletproofs::PedersenGens;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;

use crate::error::ChainError;

pub fn gens() -> PedersenGens {
    PedersenGens::default()
}

pub fn commit(value: u64, blinding: &Scalar) -> RistrettoPoint {
    gens().commit(Scalar::from(value), *blinding)
}

pub fn decompress(bytes: &[u8; 32]) -> Result<RistrettoPoint, ChainError> {
    CompressedRistretto(*bytes)
        .decompress()
        .ok_or(ChainError::Malformed)
}

/// True when inputs commit to exactly outputs plus the fee.
pub fn balances(pseudo_inputs: &[RistrettoPoint], outputs: &[RistrettoPoint], fee: u64) -> bool {
    let inputs: RistrettoPoint = pseudo_inputs.iter().sum();
    let spent: RistrettoPoint = outputs.iter().sum::<RistrettoPoint>() + commit(fee, &Scalar::ZERO);
    inputs == spent
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn commitments_add_up() {
        let (r2, r3) = (Scalar::random(&mut OsRng), Scalar::random(&mut OsRng));
        // 10 in -> 7 + 2 out, fee 1.
        let ins = [commit(10, &(r2 + r3))];
        let outs = [commit(7, &r2), commit(2, &r3)];
        assert!(balances(&ins, &outs, 1));
        assert!(!balances(&ins, &outs, 2));
    }

    #[test]
    fn hiding_depends_on_blinding() {
        let a = commit(5, &Scalar::random(&mut OsRng));
        let b = commit(5, &Scalar::random(&mut OsRng));
        assert_ne!(a, b);
    }
}
