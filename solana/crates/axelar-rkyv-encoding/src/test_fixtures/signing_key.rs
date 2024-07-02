use rand::rngs::OsRng;
use rand::Rng;

use crate::types::{PublicKey, Signature};

#[derive(Clone)]
pub enum TestSigningKey {
    Ecdsa(k256::ecdsa::SigningKey),
    Ed25519(ed25519_dalek::SigningKey),
}

impl TestSigningKey {
    pub fn sign(&self, message: &[u8]) -> Signature {
        match self {
            TestSigningKey::Ecdsa(signing_key) => {
                let (signature, recovery_id) = signing_key.sign_recoverable(message).unwrap();
                let mut signature_bytes = signature.to_vec();
                signature_bytes.push(recovery_id.to_byte());
                Signature::EcdsaRecoverable(signature_bytes.try_into().unwrap())
            }
            TestSigningKey::Ed25519(signing_key) => {
                use ed25519_dalek::Signer;
                let signature: ed25519_dalek::Signature = signing_key.sign(message);
                Signature::Ed25519(signature.to_bytes())
            }
        }
    }
}

pub fn random_keypair() -> (TestSigningKey, PublicKey) {
    if OsRng.gen_bool(0.5) {
        let signing_key = k256::ecdsa::SigningKey::random(&mut OsRng);
        let verifying_key_bytes: Box<[u8; 33]> = signing_key
            .verifying_key()
            .to_sec1_bytes()
            .try_into()
            .unwrap();
        (
            TestSigningKey::Ecdsa(signing_key),
            PublicKey::Ecdsa(*verifying_key_bytes),
        )
    } else {
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let verifying_key_bytes = signing_key.verifying_key().to_bytes();
        (
            TestSigningKey::Ed25519(signing_key),
            PublicKey::Ed25519(verifying_key_bytes),
        )
    }
}
