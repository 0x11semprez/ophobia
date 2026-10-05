//! Linkable ring signature over two rows (MLSAG).
//!
//! Row 1 proves the signer knows the secret key of one ring member's one-time key and exposes a
//! key image, unique per key, so spending the same output twice is detectable. Row 2 proves the
//! pseudo commitment hides the same amount as that member's commitment.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT as G;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use sha2::Sha512;

use super::commitment::gens;
use crate::error::ChainError;

/// A ring member as the verifier sees it.
#[derive(Clone, Copy)]
pub struct Member {
    pub key: RistrettoPoint,
    pub commitment: RistrettoPoint,
}

pub struct Signature {
    pub challenge: Scalar,
    pub s_key: Vec<Scalar>,
    pub s_amount: Vec<Scalar>,
}

fn hash_to_point(point: &RistrettoPoint) -> RistrettoPoint {
    RistrettoPoint::hash_from_bytes::<Sha512>(point.compress().as_bytes())
}

pub fn key_image(secret: &Scalar) -> RistrettoPoint {
    let public = secret * G;
    secret * hash_to_point(&public)
}

fn challenge(
    message: &[u8; 32],
    l_key: &RistrettoPoint,
    l_image: &RistrettoPoint,
    l_amount: &RistrettoPoint,
) -> Scalar {
    let mut bytes = b"ophobia/mlsag".to_vec();
    bytes.extend_from_slice(message);
    for p in [l_key, l_image, l_amount] {
        bytes.extend_from_slice(p.compress().as_bytes());
    }
    Scalar::hash_from_bytes::<Sha512>(&bytes)
}

/// One step of the ring: from challenge `c` at member `m`, the next challenge.
fn step(
    message: &[u8; 32],
    m: &Member,
    image: &RistrettoPoint,
    pseudo: &RistrettoPoint,
    c: &Scalar,
    s_key: &Scalar,
    s_amount: &Scalar,
) -> Scalar {
    let h = gens().B_blinding;
    let l_key = s_key * G + c * m.key;
    let l_image = s_key * hash_to_point(&m.key) + c * image;
    let l_amount = s_amount * h + c * (m.commitment - pseudo);
    challenge(message, &l_key, &l_image, &l_amount)
}

/// Signs as member `real`, whose one-time secret is `secret` and whose commitment differs from `pseudo`
/// by `z * B_blinding` (z = real blinding - pseudo blinding).
pub fn sign(
    message: &[u8; 32],
    ring: &[Member],
    real: usize,
    secret: &Scalar,
    z: &Scalar,
    pseudo: &RistrettoPoint,
) -> Signature {
    let n = ring.len();
    let image = key_image(secret);
    let h = gens().B_blinding;
    let alpha_key = Scalar::random(&mut OsRng);
    let alpha_amount = Scalar::random(&mut OsRng);

    let mut s_key = vec![Scalar::ZERO; n];
    let mut s_amount = vec![Scalar::ZERO; n];
    let mut c = vec![Scalar::ZERO; n];

    let start = challenge(
        message,
        &(alpha_key * G),
        &(alpha_key * hash_to_point(&ring[real].key)),
        &(alpha_amount * h),
    );
    let mut i = (real + 1) % n;
    c[i] = start;
    while i != real {
        s_key[i] = Scalar::random(&mut OsRng);
        s_amount[i] = Scalar::random(&mut OsRng);
        let next = (i + 1) % n;
        c[next] = step(
            message,
            &ring[i],
            &image,
            pseudo,
            &c[i],
            &s_key[i],
            &s_amount[i],
        );
        i = next;
    }
    s_key[real] = alpha_key - c[real] * secret;
    s_amount[real] = alpha_amount - c[real] * z;
    Signature {
        challenge: c[0],
        s_key,
        s_amount,
    }
}

pub fn verify(
    message: &[u8; 32],
    ring: &[Member],
    image: &RistrettoPoint,
    pseudo: &RistrettoPoint,
    sig: &Signature,
) -> bool {
    let n = ring.len();
    if n == 0 || sig.s_key.len() != n || sig.s_amount.len() != n {
        return false;
    }
    let mut c = sig.challenge;
    for ((member, s_key), s_amount) in ring.iter().zip(&sig.s_key).zip(&sig.s_amount) {
        c = step(message, member, image, pseudo, &c, s_key, s_amount);
    }
    c == sig.challenge
}

const SCALAR: usize = 32;

impl Signature {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.challenge.to_bytes().to_vec();
        for s in self.s_key.iter().chain(&self.s_amount) {
            out.extend_from_slice(s.as_bytes());
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ChainError> {
        if bytes.len() < SCALAR || !(bytes.len() - SCALAR).is_multiple_of(2 * SCALAR) {
            return Err(ChainError::Malformed);
        }
        let scalar = |chunk: &[u8]| {
            Option::from(Scalar::from_canonical_bytes(chunk.try_into().unwrap()))
                .ok_or(ChainError::Malformed)
        };
        let challenge = scalar(&bytes[..SCALAR])?;
        let n = (bytes.len() - SCALAR) / (2 * SCALAR);
        let rest = &bytes[SCALAR..];
        let s_key = rest[..n * SCALAR]
            .chunks(SCALAR)
            .map(scalar)
            .collect::<Result<_, _>>()?;
        let s_amount = rest[n * SCALAR..]
            .chunks(SCALAR)
            .map(scalar)
            .collect::<Result<_, _>>()?;
        Ok(Self {
            challenge,
            s_key,
            s_amount,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::commitment::commit;

    struct Fixture {
        ring: Vec<Member>,
        secret: Scalar,
        z: Scalar,
        pseudo: RistrettoPoint,
        real: usize,
    }

    fn fixture(n: usize, real: usize) -> Fixture {
        let mut ring = Vec::new();
        let mut secret = Scalar::ZERO;
        let mut blinding = Scalar::ZERO;
        for i in 0..n {
            let x = Scalar::random(&mut OsRng);
            let r = Scalar::random(&mut OsRng);
            ring.push(Member {
                key: x * G,
                commitment: commit(50, &r),
            });
            if i == real {
                (secret, blinding) = (x, r);
            }
        }
        let pseudo_blinding = Scalar::random(&mut OsRng);
        Fixture {
            ring,
            secret,
            z: blinding - pseudo_blinding,
            pseudo: commit(50, &pseudo_blinding),
            real,
        }
    }

    #[test]
    fn valid_signature_verifies_for_any_position() {
        for real in 0..4 {
            let f = fixture(4, real);
            let sig = sign(&[7; 32], &f.ring, f.real, &f.secret, &f.z, &f.pseudo);
            assert!(verify(
                &[7; 32],
                &f.ring,
                &key_image(&f.secret),
                &f.pseudo,
                &sig
            ));
        }
    }

    #[test]
    fn wrong_message_fails() {
        let f = fixture(3, 1);
        let sig = sign(&[7; 32], &f.ring, f.real, &f.secret, &f.z, &f.pseudo);
        assert!(!verify(
            &[8; 32],
            &f.ring,
            &key_image(&f.secret),
            &f.pseudo,
            &sig
        ));
    }

    #[test]
    fn pseudo_commitment_with_another_amount_fails() {
        let f = fixture(3, 0);
        let forged = commit(51, &Scalar::random(&mut OsRng));
        let sig = sign(&[1; 32], &f.ring, f.real, &f.secret, &f.z, &forged);
        assert!(!verify(
            &[1; 32],
            &f.ring,
            &key_image(&f.secret),
            &forged,
            &sig
        ));
    }

    #[test]
    fn key_image_is_stable_and_unlinkable_to_other_keys() {
        let x = Scalar::random(&mut OsRng);
        assert_eq!(key_image(&x), key_image(&x));
        assert_ne!(key_image(&x), key_image(&Scalar::random(&mut OsRng)));
    }

    #[test]
    fn wrong_key_image_fails() {
        let f = fixture(3, 2);
        let sig = sign(&[2; 32], &f.ring, f.real, &f.secret, &f.z, &f.pseudo);
        let other = key_image(&Scalar::random(&mut OsRng));
        assert!(!verify(&[2; 32], &f.ring, &other, &f.pseudo, &sig));
    }

    #[test]
    fn bytes_roundtrip() {
        let f = fixture(3, 1);
        let sig = sign(&[3; 32], &f.ring, f.real, &f.secret, &f.z, &f.pseudo);
        let back = Signature::from_bytes(&sig.to_bytes()).unwrap();
        assert!(verify(
            &[3; 32],
            &f.ring,
            &key_image(&f.secret),
            &f.pseudo,
            &back
        ));
        assert!(Signature::from_bytes(&[0; 40]).is_err());
    }
}
