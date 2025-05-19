//! Verifier set utilities that provide ability to sign over messages

use std::sync::Arc;

use axelar_solana_encoding::hasher::NativeHasher;
use axelar_solana_encoding::types::pubkey::{PublicKey, Signature};
use axelar_solana_encoding::types::verifier_set::{verifier_set_hash, VerifierSet};
use solana_sdk::pubkey::Pubkey;

/// Uitility verifier set representation that has access to the signing keys
#[derive(Clone, Debug)]
pub struct SigningVerifierSet {
    /// signers that have access to the given verifier set
    pub signers: Arc<[TestSigner]>,
    /// the nonce for the verifier set
    pub nonce: u64,
    /// quorum for the verifier set
    pub quorum: u128,
    /// the domain separator for the verifier set
    pub domain_separator: [u8; 32],
}

impl SigningVerifierSet {
    /// Create a new `SigningVerifierSet`
    ///
    /// # Panics
    /// if the calculated quorum is larger than u128
    pub fn new(signers: Arc<[TestSigner]>, nonce: u64, domain_separator: [u8; 32]) -> Self {
        let quorum = signers
            .iter()
            .map(|signer| signer.weight)
            .try_fold(0, u128::checked_add)
            .expect("no arithmetic overflow");
        Self::new_with_quorum(signers, nonce, quorum, domain_separator)
    }

    /// Create a new `SigningVerifierSet` with a custom quorum
    #[must_use]
    pub const fn new_with_quorum(
        signers: Arc<[TestSigner]>,
        nonce: u64,
        quorum: u128,
        domain_separator: [u8; 32],
    ) -> Self {
        Self {
            signers,
            nonce,
            quorum,
            domain_separator,
        }
    }

    /// Get the verifier set tracket PDA and bump
    #[must_use]
    pub fn verifier_set_tracker(&self) -> (Pubkey, u8) {
        let hash = verifier_set_hash::<NativeHasher>(&self.verifier_set(), &self.domain_separator)
            .unwrap();
        axelar_solana_gateway::get_verifier_set_tracker_pda(hash)
    }

    /// Transform into the verifier set that the gateway expects to operate on
    #[must_use]
    pub fn verifier_set(&self) -> VerifierSet {
        let signers = self
            .signers
            .iter()
            .map(|x| (x.public_key, x.weight))
            .collect();
        VerifierSet {
            nonce: self.nonce,
            signers,
            quorum: self.quorum,
        }
    }
}

/// Single test signer
#[derive(Clone, Debug)]
pub struct TestSigner {
    /// public key
    pub public_key: PublicKey,
    /// privaet key
    pub secret_key: TestSigningKey,
    /// associated weight
    pub weight: u128,
}

/// Create a new signer with the given wetight
#[must_use]
pub fn create_signer_with_weight(weight: u128) -> TestSigner {
    let (secret_key, public_key) = random_ecdsa_keypair();

    TestSigner {
        public_key,
        secret_key,
        weight,
    }
}

/// Test signer for signing payloads
#[derive(Clone)]
pub enum TestSigningKey {
    /// ECDSA key type
    Ecdsa(libsecp256k1::SecretKey),
    /// ED25519 key type
    Ed25519(ed25519_dalek::SigningKey),
}

impl TestSigningKey {
    /// Sign an arbitrary message, generating a [`Signature`]
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        match self {
            Self::Ecdsa(signing_key) => {
                let message = libsecp256k1::Message::parse(message.try_into().unwrap());
                let (signature, recovery_id) = libsecp256k1::sign(&message, signing_key);
                let mut signature_bytes = signature.serialize().to_vec();
                signature_bytes.push(recovery_id.serialize());
                Signature::EcdsaRecoverable(signature_bytes.try_into().unwrap())
            }
            Self::Ed25519(signing_key) => {
                use ed25519_dalek::Signer as _;
                let signature: ed25519_dalek::Signature = signing_key.sign(message);
                Signature::Ed25519(signature.to_bytes())
            }
        }
    }
}

/// Genetrate a random keypair
#[must_use]
pub fn random_keypair() -> (TestSigningKey, PublicKey) {
    // NOTE: the Gateway can't process Ed25519 signatures yet, so this function will
    // issue only ECDSA keypairs for now.
    //
    // if OsRng.gen_bool(0.5) {
    random_ecdsa_keypair()
    // } else {
    //     random_ed25519_keypair()
    // }
}

/// New random ED25519 keypair
pub fn random_ed25519_keypair() -> (TestSigningKey, PublicKey) {
    let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
    let verifying_key_bytes = signing_key.verifying_key().to_bytes();
    (
        TestSigningKey::Ed25519(signing_key),
        PublicKey::Ed25519(verifying_key_bytes),
    )
}

/// New random ECDSA keypair
pub fn random_ecdsa_keypair() -> (TestSigningKey, PublicKey) {
    let signing_key = libsecp256k1::SecretKey::random(&mut libsecp_rand::rngs::OsRng);
    let public_key = libsecp256k1::PublicKey::from_secret_key(&signing_key);
    (
        TestSigningKey::Ecdsa(signing_key),
        PublicKey::Secp256k1(public_key.serialize_compressed()),
    )
}

#[allow(clippy::min_ident_chars)]
impl core::fmt::Debug for TestSigningKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Ecdsa(_) => f.write_str("ecdsa<pk>"),
            Self::Ed25519(_) => f.write_str("ed25519<pk>"),
        }
    }
}
