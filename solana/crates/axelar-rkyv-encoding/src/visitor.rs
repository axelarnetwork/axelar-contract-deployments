#[cfg(test)]
use crate::types::ExecuteData;
use crate::types::{
    ArchivedCrossChainId, ArchivedExecuteData, ArchivedHasheableMessageVec, ArchivedMessage,
    ArchivedPayload, ArchivedProof, ArchivedPublicKey, ArchivedSignature, ArchivedU128,
    ArchivedVerifierSet, ArchivedWeightedSigner, CrossChainId, HasheableMessageVec, Message,
    Payload, Proof, PublicKey, Signature, VerifierSet, WeightedSigner, U128,
};

const CHAIN_NAME_DELIMITER: &[u8] = b"-";

pub trait Visitor<'a> {
    #[cfg(test)]
    fn visit_execute_data(&mut self, execute_data: &'a ExecuteData) {
        let ExecuteData { payload, proof } = execute_data;
        self.visit_payload(payload);
        self.visit_proof(proof);
    }

    fn visit_proof(&mut self, proof: &'a Proof) {
        let Proof {
            signers_with_signatures,
            threshold,
            ..
        } = proof;

        self.prefix_length(signers_with_signatures.len_be_bytes());
        for signature in signers_with_signatures.iter() {
            self.visit_weighted_signature(signature.0, signature.1);
        }
        self.visit_u128(threshold);
        self.visit_u64(proof.nonce_be_bytes());
    }

    fn visit_weighted_signature(&mut self, pubkey: &'a PublicKey, signature: &'a WeightedSigner) {
        let WeightedSigner { signature, weight } = signature;
        self.visit_public_key(pubkey);
        if let Some(signature) = signature {
            self.visit_signature(signature);
        }
        self.visit_u128(weight);
    }

    fn visit_signature(&mut self, signature: &'a Signature) {
        match signature {
            Signature::EcdsaRecoverable(signature) => {
                self.tag(b"ecdsa-sig");
                self.visit_bytes(signature);
            }
            Signature::Ed25519(signature) => {
                self.tag(b"ed25519-sig");
                self.visit_bytes(signature);
            }
        }
    }

    fn visit_payload(&mut self, payload: &'a Payload) {
        match payload {
            Payload::Messages(messages) => {
                self.tag(b"messages");
                self.visit_messages(messages)
            }
            Payload::VerifierSet(verifier_set) => {
                self.tag(b"verifier_set");
                self.visit_verifier_set(verifier_set)
            }
        }
    }

    fn visit_messages(&mut self, messages: &'a HasheableMessageVec) {
        self.prefix_length(messages.len_be_bytes());
        for message in messages.iter() {
            self.visit_message(message);
        }
    }

    fn visit_message(&mut self, message: &'a Message) {
        self.visit_cc_id(&message.cc_id);
        self.visit_string(message.source_address.as_str());
        self.visit_string(message.destination_chain.as_ref());
        self.visit_string(message.destination_address.as_str());
        self.visit_bytes(&message.payload_hash);
    }

    /// Visit Message's CCID following its `Display` implementation.
    fn visit_cc_id(&mut self, cc_id: &'a CrossChainId) {
        self.visit_string(cc_id.chain.as_ref());
        self.visit_bytes(CHAIN_NAME_DELIMITER);
        self.visit_string(&cc_id.id);
    }

    fn visit_verifier_set(&mut self, verifier_set: &'a VerifierSet) {
        self.prefix_length(verifier_set.signers.len_be_bytes());
        for (public_key, weight) in verifier_set.signers.iter() {
            self.visit_public_key(public_key);
            self.visit_u128(weight);
        }
        self.visit_u128(&verifier_set.quorum);
        self.visit_u64(verifier_set.created_at_be_bytes())
    }

    fn visit_public_key(&mut self, public_key: &'a PublicKey) {
        match public_key {
            PublicKey::Secp256k1(pubkey_bytes) => {
                self.tag(b"secp256k1");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
            PublicKey::Ed25519(pubkey_bytes) => {
                self.tag(b"ed25519");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
        }
    }

    fn visit_string(&mut self, string: &'a str) {
        self.visit_bytes(string.as_bytes())
    }

    fn visit_u64(&mut self, number: &'a [u8]) {
        self.visit_bytes(number)
    }

    fn visit_u128(&mut self, number: &'a U128) {
        self.visit_bytes(number.to_le())
    }

    fn visit_bytes(&mut self, bytes: &'a [u8]);

    fn prefix_length(&mut self, length: &'a [u8]) {
        self.tag(length)
    }

    /// No-op by default.
    fn tag(&mut self, _bytes: &'a [u8]) {}
}

pub trait ArchivedVisitor<'a> {
    fn visit_execute_data(&mut self, execute_data: &'a ArchivedExecuteData) {
        let ArchivedExecuteData { payload, proof } = execute_data;
        self.visit_payload(payload);
        self.visit_proof(proof);
    }

    fn visit_proof(&mut self, proof: &'a ArchivedProof) {
        let ArchivedProof {
            signers_with_signatures,
            threshold,
            ..
        } = proof;
        self.prefix_length(signers_with_signatures.len_be_bytes());
        for (pubkey, signature) in signers_with_signatures.iter() {
            self.visit_weighted_signature(pubkey, signature);
        }
        self.visit_u128(threshold);
        self.visit_u64(proof.nonce_be_bytes());
    }

    fn visit_weighted_signature(
        &mut self,
        public_key: &'a ArchivedPublicKey,
        signature: &'a ArchivedWeightedSigner,
    ) {
        let ArchivedWeightedSigner { signature, weight } = signature;
        self.visit_public_key(public_key);

        if let Some(signature) = signature.as_ref() {
            self.visit_signature(signature);
        }
        self.visit_u128(weight);
    }

    fn visit_signature(&mut self, signature: &'a ArchivedSignature) {
        match signature {
            ArchivedSignature::EcdsaRecoverable(signature) => {
                self.tag(b"ecdsa-sig");
                self.visit_bytes(signature);
            }
            ArchivedSignature::Ed25519(signature) => {
                self.tag(b"ed25519-sig");
                self.visit_bytes(signature);
            }
        }
    }

    fn visit_payload(&mut self, payload: &'a ArchivedPayload) {
        match payload {
            ArchivedPayload::Messages(messages) => {
                self.tag(b"messages");
                self.visit_messages(messages)
            }
            ArchivedPayload::VerifierSet(verifier_set) => {
                self.tag(b"verifier_set");
                self.visit_verifier_set(verifier_set)
            }
        }
    }

    fn visit_messages(&mut self, messages: &'a ArchivedHasheableMessageVec) {
        self.prefix_length(messages.len_be_bytes());
        for message in messages.iter() {
            self.visit_message(message);
        }
    }

    fn visit_message(&mut self, message: &'a ArchivedMessage) {
        self.visit_cc_id(&message.cc_id);
        self.visit_string(message.source_address.as_str());
        self.visit_string(message.destination_chain.as_ref());
        self.visit_string(message.destination_address.as_str());
        self.visit_bytes(&message.payload_hash);
    }

    /// Visit Message's CCID following its `Display` implementation.
    fn visit_cc_id(&mut self, cc_id: &'a ArchivedCrossChainId) {
        self.visit_string(cc_id.chain.as_ref());
        self.visit_bytes(CHAIN_NAME_DELIMITER);
        self.visit_string(&cc_id.id);
    }

    fn visit_verifier_set(&mut self, verifier_set: &'a ArchivedVerifierSet) {
        self.prefix_length(verifier_set.signers.len_be_bytes());
        for (public_key, weight) in verifier_set.signers.iter() {
            self.visit_public_key(public_key);
            self.visit_u128(weight);
        }
        self.visit_u128(&verifier_set.quorum);
        self.visit_u64(verifier_set.created_at_be_bytes())
    }

    fn visit_public_key(&mut self, public_key: &'a ArchivedPublicKey) {
        match public_key {
            ArchivedPublicKey::Secp256k1(pubkey_bytes) => {
                self.tag(b"secp256k1");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
            ArchivedPublicKey::Ed25519(pubkey_bytes) => {
                self.tag(b"ed25519");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
        }
    }

    fn visit_string(&mut self, string: &'a str) {
        self.visit_bytes(string.as_bytes())
    }

    fn visit_u64(&mut self, number: &'a [u8]) {
        self.visit_bytes(number)
    }

    fn visit_u128(&mut self, number: &'a ArchivedU128) {
        self.visit_bytes(number.to_le())
    }

    fn visit_bytes(&mut self, bytes: &'a [u8]);

    fn prefix_length(&mut self, length: &'a [u8]) {
        self.tag(length)
    }

    /// No-op by default.
    fn tag(&mut self, _bytes: &'a [u8]) {}
}
