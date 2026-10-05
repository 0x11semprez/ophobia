//! Stealth addresses: the sender derives a one-time key per output that only the recipient can link to their address.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use sha2::Sha512;

use super::commitment::decompress;
use crate::error::ChainError;

/// Recipient secrets: a view key to find outputs and a spend key to spend them.
pub struct Wallet {
    view: Scalar,
    spend: Scalar,
}

/// Public address published by the recipient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address {
    pub view: [u8; 32],
    pub spend: [u8; 32],
}

/// What the sender writes into an output.
pub struct Stealth {
    pub one_time_key: [u8; 32],
    pub ephemeral_key: [u8; 32],
    pub blinding: Scalar,
    pub encrypted_amount: [u8; 8],
}

/// What the recipient learns from an output that is theirs.
pub struct Received {
    pub secret_key: Scalar,
    pub blinding: Scalar,
    pub amount: u64,
}

fn hash_to_scalar(label: &[u8], point: &RistrettoPoint, index: u64) -> Scalar {
    let mut bytes = label.to_vec();
    bytes.extend_from_slice(point.compress().as_bytes());
    bytes.extend_from_slice(&index.to_le_bytes());
    Scalar::hash_from_bytes::<Sha512>(&bytes)
}

fn mask(shared: &RistrettoPoint, index: u64) -> [u8; 8] {
    let s = hash_to_scalar(b"ophobia/amount", shared, index);
    s.to_bytes()[..8].try_into().unwrap()
}

impl Wallet {
    pub fn generate() -> Self {
        Self {
            view: Scalar::random(&mut OsRng),
            spend: Scalar::random(&mut OsRng),
        }
    }

    pub fn address(&self) -> Address {
        Address {
            view: (self.view * G).compress().to_bytes(),
            spend: (self.spend * G).compress().to_bytes(),
        }
    }

    /// Returns the output's secrets when it pays this wallet, `None` otherwise.
    pub fn scan(
        &self,
        one_time_key: &[u8; 32],
        ephemeral_key: &[u8; 32],
        encrypted_amount: &[u8; 8],
        index: u64,
    ) -> Option<Received> {
        let ephemeral = decompress(ephemeral_key).ok()?;
        let shared = self.view * ephemeral;
        let secret_key = hash_to_scalar(b"ophobia/key", &shared, index) + self.spend;
        if (secret_key * G).compress().to_bytes() != *one_time_key {
            return None;
        }
        let blinding = hash_to_scalar(b"ophobia/blind", &shared, index);
        let mut amount = [0u8; 8];
        for (out, (c, m)) in amount
            .iter_mut()
            .zip(encrypted_amount.iter().zip(mask(&shared, index)))
        {
            *out = c ^ m;
        }
        Some(Received {
            secret_key,
            blinding,
            amount: u64::from_le_bytes(amount),
        })
    }
}

/// Derives the one-time key, blinding factor and masked amount for output `index` paying `to`.
pub fn derive(to: &Address, amount: u64, index: u64) -> Result<Stealth, ChainError> {
    let view = decompress(&to.view)?;
    let spend = decompress(&to.spend)?;
    let r = Scalar::random(&mut OsRng);
    let shared = r * view;
    let one_time = hash_to_scalar(b"ophobia/key", &shared, index) * G + spend;
    let mut masked = [0u8; 8];
    for (out, (a, m)) in masked
        .iter_mut()
        .zip(amount.to_le_bytes().iter().zip(mask(&shared, index)))
    {
        *out = a ^ m;
    }
    Ok(Stealth {
        one_time_key: one_time.compress().to_bytes(),
        ephemeral_key: (r * G).compress().to_bytes(),
        blinding: hash_to_scalar(b"ophobia/blind", &shared, index),
        encrypted_amount: masked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_recovers_amount_and_key() {
        let wallet = Wallet::generate();
        let s = derive(&wallet.address(), 4242, 3).unwrap();
        let got = wallet
            .scan(&s.one_time_key, &s.ephemeral_key, &s.encrypted_amount, 3)
            .unwrap();
        assert_eq!(got.amount, 4242);
        assert_eq!(got.blinding, s.blinding);
        assert_eq!((got.secret_key * G).compress().to_bytes(), s.one_time_key);
    }

    #[test]
    fn other_wallet_sees_nothing() {
        let s = derive(&Wallet::generate().address(), 1, 0).unwrap();
        assert!(
            Wallet::generate()
                .scan(&s.one_time_key, &s.ephemeral_key, &s.encrypted_amount, 0)
                .is_none()
        );
    }

    #[test]
    fn index_binds_the_output() {
        let wallet = Wallet::generate();
        let s = derive(&wallet.address(), 1, 0).unwrap();
        assert!(
            wallet
                .scan(&s.one_time_key, &s.ephemeral_key, &s.encrypted_amount, 1)
                .is_none()
        );
    }
}
