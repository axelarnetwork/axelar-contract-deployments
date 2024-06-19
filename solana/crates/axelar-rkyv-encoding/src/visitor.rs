use crate::types::{
    ArchivedCrossChainId, ArchivedMessage, ArchivedPayload, ArchivedPublicKey, ArchivedSigner,
    ArchivedU256, ArchivedVerifierSet, CrossChainId, Message, Payload, PublicKey, Signer,
    VerifierSet, U256,
};
#[cfg(test)]
use crate::types::{
    ArchivedExecuteData, ArchivedProof, ArchivedSignature, ArchivedWeightedSignature, ExecuteData,
    Proof, Signature, WeightedSignature,
};

const CHAIN_NAME_DELIMITER: &[u8] = b"-";

pub(super) trait Visitor {
    #[cfg(test)]
    fn visit_execute_data(&mut self, execute_data: &ExecuteData) {
        let ExecuteData { payload, proof } = execute_data;
        self.visit_payload(payload);
        self.visit_proof(proof);
    }

    #[cfg(test)]
    fn visit_proof(&mut self, proof: &Proof) {
        let Proof {
            signatures,
            threshold,
            nonce,
        } = proof;

        self.prefix_length(signatures.len());
        for signature in signatures {
            self.visit_weighted_signature(signature);
        }
        self.visit_u256(threshold);
        self.visit_u64(nonce);
    }

    #[cfg(test)]
    fn visit_weighted_signature(&mut self, signature: &WeightedSignature) {
        let WeightedSignature {
            pubkey,
            signature,
            weight,
        } = signature;
        self.visit_public_key(pubkey);
        self.visit_signature(signature);
        self.visit_u256(weight);
    }

    #[cfg(test)]
    fn visit_signature(&mut self, signature: &Signature) {
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

    fn visit_payload(&mut self, payload: &Payload) {
        match payload {
            Payload::Messages(messages) => {
                self.tag(b"messages");
                self.prefix_length(messages.len());
                for message in messages {
                    self.visit_message(message);
                }
            }
            Payload::VerifierSet(verifier_set) => {
                self.tag(b"verifier_set");
                self.visit_verifier_set(verifier_set)
            }
        }
    }

    fn visit_message(&mut self, message: &Message) {
        self.visit_cc_id(&message.cc_id);
        self.visit_string(message.source_address.as_str());
        self.visit_string(message.destination_chain.as_ref());
        self.visit_string(message.destination_address.as_str());
        self.visit_bytes(&message.payload_hash);
    }

    /// Visit Message's CCID following its `Display` implementation.
    fn visit_cc_id(&mut self, cc_id: &CrossChainId) {
        self.visit_string(cc_id.chain.as_ref());
        self.visit_bytes(CHAIN_NAME_DELIMITER);
        self.visit_string(&cc_id.id);
    }

    fn visit_verifier_set(&mut self, verifier_set: &VerifierSet) {
        self.prefix_length(verifier_set.signers.len());
        for signer in verifier_set.signers.values() {
            self.visit_signer(signer);
        }
        self.visit_u256(&verifier_set.threshold);
        self.visit_u64(&verifier_set.created_at)
    }

    fn visit_public_key(&mut self, public_key: &PublicKey) {
        match public_key {
            PublicKey::Ecdsa(pubkey_bytes) => {
                self.tag(b"ecdsa");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
            PublicKey::Ed25519(pubkey_bytes) => {
                self.tag(b"ed25519");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
        }
    }

    fn visit_signer(&mut self, signer: &Signer) {
        self.visit_string(&signer.address);
        self.visit_u256(&signer.weight);
        self.visit_public_key(&signer.public_key)
    }

    fn visit_string(&mut self, string: &str) {
        self.visit_bytes(string.as_bytes())
    }

    fn visit_u64(&mut self, number: &u64) {
        self.visit_bytes(&number.to_be_bytes())
    }

    fn visit_u256(&mut self, number: &U256) {
        self.visit_bytes(&number.to_le())
    }

    fn visit_bytes(&mut self, bytes: &[u8]);

    fn prefix_length(&mut self, length: usize) {
        self.tag(&(length as u64).to_be_bytes())
    }

    /// No-op by default.
    fn tag(&mut self, _bytes: &[u8]) {}
}

pub(super) trait ArchivedVisitor {
    #[cfg(test)]
    fn visit_execute_data(&mut self, execute_data: &ArchivedExecuteData) {
        let ArchivedExecuteData { payload, proof } = execute_data;
        self.visit_payload(payload);
        self.visit_proof(proof);
    }

    #[cfg(test)]
    fn visit_proof(&mut self, proof: &ArchivedProof) {
        let ArchivedProof {
            signatures,
            threshold,
            nonce,
        } = proof;
        self.prefix_length(signatures.len());
        for signature in signatures.iter() {
            self.visit_weighted_signature(signature);
        }
        self.visit_u256(threshold);
        self.visit_u64(nonce);
    }
    #[cfg(test)]
    fn visit_weighted_signature(&mut self, signature: &ArchivedWeightedSignature) {
        let ArchivedWeightedSignature {
            pubkey,
            signature,
            weight,
        } = signature;
        self.visit_public_key(pubkey);
        self.visit_signature(signature);
        self.visit_u256(weight);
    }
    #[cfg(test)]
    fn visit_signature(&mut self, signature: &ArchivedSignature) {
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

    fn visit_payload(&mut self, payload: &ArchivedPayload) {
        match payload {
            ArchivedPayload::Messages(messages) => {
                self.tag(b"messages");
                self.prefix_length(messages.len());
                for message in messages.iter() {
                    self.visit_message(message);
                }
            }
            ArchivedPayload::VerifierSet(verifier_set) => {
                self.tag(b"verifier_set");
                self.visit_verifier_set(verifier_set)
            }
        }
    }

    fn visit_message(&mut self, message: &ArchivedMessage) {
        self.visit_cc_id(&message.cc_id);
        self.visit_string(message.source_address.as_str());
        self.visit_string(message.destination_chain.as_ref());
        self.visit_string(message.destination_address.as_str());
        self.visit_bytes(&message.payload_hash);
    }

    /// Visit Message's CCID following its `Display` implementation.
    fn visit_cc_id(&mut self, cc_id: &ArchivedCrossChainId) {
        self.visit_string(cc_id.chain.as_ref());
        self.visit_bytes(CHAIN_NAME_DELIMITER);
        self.visit_string(&cc_id.id);
    }

    fn visit_verifier_set(&mut self, verifier_set: &ArchivedVerifierSet) {
        self.prefix_length(verifier_set.signers.len());
        for signer in verifier_set.signers.values() {
            self.visit_signer(signer);
        }
        self.visit_u256(&verifier_set.threshold);
        self.visit_u64(&verifier_set.created_at)
    }

    fn visit_public_key(&mut self, public_key: &ArchivedPublicKey) {
        match public_key {
            ArchivedPublicKey::Ecdsa(pubkey_bytes) => {
                self.tag(b"ecdsa");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
            ArchivedPublicKey::Ed25519(pubkey_bytes) => {
                self.tag(b"ed25519");
                self.visit_bytes(pubkey_bytes.as_slice())
            }
        }
    }

    fn visit_signer(&mut self, signer: &ArchivedSigner) {
        self.visit_string(&signer.address);
        self.visit_u256(&signer.weight);
        self.visit_public_key(&signer.public_key)
    }

    fn visit_string(&mut self, string: &str) {
        self.visit_bytes(string.as_bytes())
    }

    fn visit_u64(&mut self, number: &u64) {
        self.visit_bytes(&number.to_be_bytes())
    }

    fn visit_u256(&mut self, number: &ArchivedU256) {
        self.visit_bytes(number.to_le())
    }

    fn visit_bytes(&mut self, bytes: &[u8]);

    fn prefix_length(&mut self, length: usize) {
        self.tag(&(length as u64).to_be_bytes())
    }

    /// No-op by default.
    fn tag(&mut self, _bytes: &[u8]) {}
}
