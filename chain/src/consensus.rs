use crate::error::ChainError;
use crate::types::Header;

/// Rule deciding which headers are acceptable and how to produce one.
pub trait Consensus {
    /// Finds the header fields that make `header` acceptable.
    fn seal(&self, header: &mut Header);
    /// Checks that `header` respects the rule.
    fn verify(&self, header: &Header) -> Result<(), ChainError>;
}

/// Proof of work: the header hash must start with `difficulty_bits` zero bits.
#[derive(Clone, Copy, Debug)]
pub struct ProofOfWork {
    pub difficulty_bits: u32,
}

fn leading_zero_bits(hash: &[u8; 32]) -> u32 {
    let mut bits = 0;
    for byte in hash {
        if *byte == 0 {
            bits += 8;
        } else {
            return bits + byte.leading_zeros();
        }
    }
    bits
}

impl Consensus for ProofOfWork {
    fn seal(&self, header: &mut Header) {
        header.nonce = 0;
        while leading_zero_bits(&header.hash()) < self.difficulty_bits {
            header.nonce += 1;
        }
    }

    fn verify(&self, header: &Header) -> Result<(), ChainError> {
        if leading_zero_bits(&header.hash()) >= self.difficulty_bits {
            Ok(())
        } else {
            Err(ChainError::BadBlock("insufficient proof of work"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Header {
        Header {
            height: 1,
            prev_hash: [0; 32],
            tx_root: [0; 32],
            timestamp: 1,
            nonce: 0,
        }
    }

    #[test]
    fn sealed_header_verifies() {
        let pow = ProofOfWork {
            difficulty_bits: 10,
        };
        let mut h = header();
        pow.seal(&mut h);
        assert!(pow.verify(&h).is_ok());
    }

    #[test]
    fn harder_rule_rejects_easy_header() {
        let mut h = header();
        ProofOfWork { difficulty_bits: 4 }.seal(&mut h);
        // 4 leading zero bits is not 24, except with probability 2^-20 for this nonce.
        assert!(
            ProofOfWork {
                difficulty_bits: 24
            }
            .verify(&h)
            .is_err()
        );
    }

    #[test]
    fn counts_leading_zero_bits() {
        let mut hash = [0u8; 32];
        hash[1] = 0b0010_0000;
        assert_eq!(leading_zero_bits(&hash), 10);
        assert_eq!(leading_zero_bits(&[0; 32]), 256);
    }
}
