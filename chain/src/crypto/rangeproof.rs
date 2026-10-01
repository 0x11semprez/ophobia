//! Bulletproofs proving a committed amount fits in 64 bits, so no output can wrap the sum.

use std::sync::LazyLock;

use bulletproofs::{BulletproofGens, RangeProof};
use curve25519_dalek::ristretto::CompressedRistretto;
use curve25519_dalek::scalar::Scalar;
use merlin::Transcript;

use super::commitment::gens;
use crate::error::ChainError;

const BITS: usize = 64;
const DOMAIN: &[u8] = b"ophobia/range-proof";

static BP_GENS: LazyLock<BulletproofGens> = LazyLock::new(|| BulletproofGens::new(BITS, 1));

/// Returns the proof and the compressed commitment it proves.
pub fn prove(value: u64, blinding: &Scalar) -> Result<(Vec<u8>, [u8; 32]), ChainError> {
    let mut transcript = Transcript::new(DOMAIN);
    let (proof, commitment) =
        RangeProof::prove_single(&BP_GENS, &gens(), &mut transcript, value, blinding, BITS)
            .map_err(|_| ChainError::BadRangeProof)?;
    Ok((proof.to_bytes(), commitment.to_bytes()))
}

pub fn verify(commitment: &[u8; 32], proof: &[u8]) -> bool {
    let Ok(proof) = RangeProof::from_bytes(proof) else {
        return false;
    };
    let mut transcript = Transcript::new(DOMAIN);
    proof
        .verify_single(&BP_GENS, &gens(), &mut transcript, &CompressedRistretto(*commitment), BITS)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::commitment::commit;
    use rand::rngs::OsRng;

    #[test]
    fn valid_proof_verifies() {
        let r = Scalar::random(&mut OsRng);
        let (proof, c) = prove(1234, &r).unwrap();
        assert_eq!(c, commit(1234, &r).compress().to_bytes());
        assert!(verify(&c, &proof));
    }

    #[test]
    fn proof_does_not_fit_another_commitment() {
        let (proof, _) = prove(1, &Scalar::random(&mut OsRng)).unwrap();
        let other = commit(2, &Scalar::random(&mut OsRng)).compress().to_bytes();
        assert!(!verify(&other, &proof));
    }

    #[test]
    fn truncated_proof_is_rejected() {
        let r = Scalar::random(&mut OsRng);
        let (proof, c) = prove(9, &r).unwrap();
        assert!(!verify(&c, &proof[..proof.len() - 1]));
    }
}
