use rand::rngs::OsRng;

use crate::types::{PublicKey, Signature};

#[derive(Clone)]
pub enum TestSigningKey {
    Ecdsa(libsecp256k1::SecretKey),
    Ed25519(ed25519_dalek::SigningKey),
}

impl TestSigningKey {
    pub fn sign(&self, message: &[u8]) -> Signature {
        match self {
            TestSigningKey::Ecdsa(signing_key) => {
                let message = libsecp256k1::Message::parse(message.try_into().unwrap());
                let (signature, recovery_id) = libsecp256k1::sign(&message, signing_key);
                let mut signature_bytes = signature.serialize().to_vec();
                signature_bytes.push(recovery_id.serialize());
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
    // todo: try out mixed keypairs after we add support for them on the gateway
    random_ecdsa_keypair()
    // if OsRng.gen_bool(0.5) {
    //     random_ecdsa_keypair()
    // } else {
    //     random_ed25519_keypair()
    // }
}

pub fn random_ed25519_keypair() -> (TestSigningKey, PublicKey) {
    let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let verifying_key_bytes = signing_key.verifying_key().to_bytes();
    (
        TestSigningKey::Ed25519(signing_key),
        PublicKey::Ed25519(verifying_key_bytes),
    )
}

pub fn random_ecdsa_keypair() -> (TestSigningKey, PublicKey) {
    let signing_key = libsecp256k1::SecretKey::random(&mut libsecp_rand::rngs::OsRng);
    let public_key = libsecp256k1::PublicKey::from_secret_key(&signing_key);
    (
        TestSigningKey::Ecdsa(signing_key),
        PublicKey::Secp256k1(public_key.serialize_compressed()),
    )
}

impl std::fmt::Debug for TestSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestSigningKey::Ecdsa(_) => f.write_str("ecdsa<pk>"),
            TestSigningKey::Ed25519(_) => f.write_str("ed25519<pk>"),
        }
    }
}
