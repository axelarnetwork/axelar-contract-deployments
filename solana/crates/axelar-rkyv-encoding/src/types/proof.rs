use std::collections::BTreeMap;

use rkyv::bytecheck::{self, CheckBytes};
use rkyv::collections::ArchivedBTreeMap;
use rkyv::{Archive, Deserialize, Serialize};

use super::{
    ArchivedPublicKey, Ed25519Pubkey, HasheableSignersWithSignaturesBTreeMap, PublicKey,
    Secp256k1Pubkey, VerifierSet,
};
use crate::hasher::AxelarRkyv256Hasher;
use crate::types::{
    ArchivedWeightedSigner, EcdsaRecoverableSignature, Ed25519Signature, WeightedSigner, U128,
};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct Proof {
    pub signers_with_signatures: HasheableSignersWithSignaturesBTreeMap,
    pub threshold: U128,
    pub nonce: u64,
    nonce_le_bytes: [u8; 8],
}

impl Proof {
    pub fn new(
        signers_with_signatures: BTreeMap<PublicKey, WeightedSigner>,
        threshold: U128,
        nonce: u64,
    ) -> Self {
        Self {
            signers_with_signatures: HasheableSignersWithSignaturesBTreeMap::new(
                signers_with_signatures,
            ),
            threshold,
            nonce,
            nonce_le_bytes: nonce.to_le_bytes(),
        }
    }

    pub fn nonce_le_bytes(&self) -> &[u8; 8] {
        &self.nonce_le_bytes
    }

    pub fn verifier_set(&self, domain_separator: [u8; 32]) -> VerifierSet {
        let signers = self
            .signers_with_signatures
            .iter()
            .map(|(pubkey, signer)| (*pubkey, signer.weight))
            .collect();
        VerifierSet::new(self.nonce, signers, self.threshold, domain_separator)
    }
}

impl ArchivedProof {
    /// Returns the same hash of an equivalent `VerifierSet`.
    pub fn signer_set_hash<'a>(
        &'a self,
        mut hasher_impl: impl AxelarRkyv256Hasher<'a>,
        domain_separator: &'a [u8; 32],
    ) -> [u8; 32] {
        self.drive_visitor_for_signer_set_hash(&mut hasher_impl, domain_separator);
        hasher_impl.result().into()
    }

    pub(crate) fn drive_visitor_for_signer_set_hash<'a>(
        &'a self,
        visitor: &mut impl crate::visitor::ArchivedVisitor<'a>,
        domain_separator: &'a [u8; 32],
    ) {
        // Follow `ArchivedVisitor::visit_verifier_set` exact steps
        visitor.prefix_length(self.signers_with_signatures.len_le_bytes());
        for (pubkey, weighted_signature) in self.signers_with_signatures.iter() {
            visitor.visit_public_key(pubkey);
            visitor.visit_u128(&weighted_signature.weight);
        }
        visitor.visit_u128(&self.threshold);
        visitor.visit_u64(self.nonce_le_bytes());
        visitor.visit_bytes(domain_separator);
    }

    pub fn validate_for_message(&self, message: &[u8; 32]) -> Result<(), MessageValidationError> {
        fn verify_ecdsa(
            pubkey: &Secp256k1Pubkey,
            signature: &EcdsaRecoverableSignature,
            message: &[u8; 32],
        ) -> bool {
            ArchivedWeightedSigner::verify_ecdsa(signature, pubkey, message).is_ok()
        }

        fn verify_eddsa(
            pubkey: &Ed25519Pubkey,
            signature: &Ed25519Signature,
            message: &[u8; 32],
        ) -> bool {
            ArchivedWeightedSigner::verify_ed25519(signature, pubkey, message).is_ok()
        }
        Self::validate_for_message_custom(self, message, verify_ecdsa, verify_eddsa)
    }

    pub fn validate_for_message_custom<F, G>(
        &self,
        message: &[u8; 32],
        verify_ecdsa_signature: F,
        verify_eddsa_signature: G,
    ) -> Result<(), MessageValidationError>
    where
        F: Fn(&Secp256k1Pubkey, &EcdsaRecoverableSignature, &[u8; 32]) -> bool,
        G: Fn(&Ed25519Pubkey, &Ed25519Signature, &[u8; 32]) -> bool,
    {
        use crate::types::ArchivedSignature;
        let threshold: bnum::types::U128 = (&self.threshold).into();
        let mut total_weight = bnum::types::U128::ZERO;
        for (pubkey, signer) in self.signers_with_signatures.iter() {
            // Signature validation is deferred to the caller.
            let valid_signature: bool = match (&signer.signature.as_ref(), &pubkey) {
                (
                    Some(ArchivedSignature::EcdsaRecoverable(sig)),
                    ArchivedPublicKey::Secp256k1(pubkey),
                ) => verify_ecdsa_signature(pubkey, sig, message),
                (Some(ArchivedSignature::Ed25519(sig)), ArchivedPublicKey::Ed25519(pubkey)) => {
                    verify_eddsa_signature(pubkey, sig, message)
                }
                // if the signature is not present the we just skip it
                (None, _) => continue,
                // Provided ed25519 + ecdsa combo, which is invalid state
                _ => unreachable!(),
            };
            if !valid_signature {
                return Err(MessageValidationError::InvalidSignature);
            }

            // Accumulate signer weight.
            let signer_weight = &signer.weight;
            total_weight = total_weight
                .checked_add(signer_weight.into())
                .ok_or(MessageValidationError::ArithmeticOverflow)?;
            // Return as soon as threshold is hit.
            if total_weight >= threshold {
                return Ok(());
            }
        }
        Err(MessageValidationError::InsufficientWeight)
    }

    pub fn signers_with_signatures(
        &self,
    ) -> &ArchivedBTreeMap<ArchivedPublicKey, ArchivedWeightedSigner> {
        &self.signers_with_signatures
    }

    pub fn nonce_le_bytes(&self) -> &[u8; 8] {
        &self.nonce_le_bytes
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MessageValidationError {
    #[error("Signature verification failed")]
    InvalidSignature,
    #[error("Arithmetic overflow when summing weights")]
    ArithmeticOverflow,
    #[error("Insufficient signer weight")]
    InsufficientWeight,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{
        random_valid_proof_and_message, random_valid_proof_message_and_verifier_set,
        test_hasher_impl,
    };

    fn verify_ecdsa(
        pubkey: &Secp256k1Pubkey,
        signature: &EcdsaRecoverableSignature,
        message: &[u8; 32],
    ) -> bool {
        ArchivedWeightedSigner::verify_ecdsa(signature, pubkey, message).is_ok()
    }

    fn verify_eddsa(
        pubkey: &Ed25519Pubkey,
        signature: &Ed25519Signature,
        message: &[u8; 32],
    ) -> bool {
        ArchivedWeightedSigner::verify_ed25519(signature, pubkey, message).is_ok()
    }

    #[test]
    fn valid_proof() {
        let (proof, message) = random_valid_proof_and_message::<32>();
        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        assert!(proof
            .validate_for_message_custom(&message, verify_ecdsa, verify_eddsa)
            .is_ok())
    }

    #[test]
    fn invalid_proof_insufficient_weight() {
        let (mut proof, message) = random_valid_proof_and_message::<32>();

        // Fixture Proof threshold values are always equal to the sum of signer weights.
        // Let's bump that.
        proof.threshold = bnum::types::U128::ONE
            .checked_add(proof.threshold.into())
            .unwrap()
            .into();

        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        assert!(matches!(
            proof
                .validate_for_message_custom(&message, verify_ecdsa, verify_eddsa)
                .unwrap_err(),
            MessageValidationError::InsufficientWeight
        ))
    }

    #[test]
    fn invalid_proof_wrong_message() {
        let (proof, mut message) = random_valid_proof_and_message::<32>();
        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        // Flip the first bit of the message.
        message[0] ^= 1;

        assert!(matches!(
            proof
                .validate_for_message_custom(&message, verify_ecdsa, verify_eddsa)
                .unwrap_err(),
            MessageValidationError::InvalidSignature
        ))
    }

    #[test]
    fn exact_hash_of_equivalent_signer_set() {
        let (proof, message, verifier_set) = random_valid_proof_message_and_verifier_set::<32>();

        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        assert!(proof
            .validate_for_message_custom(&message, verify_ecdsa, verify_eddsa)
            .is_ok()); // Confidence check
        assert_eq!(
            proof.signer_set_hash(test_hasher_impl(), &verifier_set.domain_separator),
            verifier_set.hash(test_hasher_impl())
        );
    }
}
