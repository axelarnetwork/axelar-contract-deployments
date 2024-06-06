use crate::types::{CrossChainId, Message, Payload, PublicKey, Signer, WorkerSet, U256};

const CHAIN_NAME_DELIMITER: &[u8] = b"-";

pub(super) trait Visitor {
    fn visit_payload(&mut self, payload: &Payload) {
        match payload {
            Payload::Messages(messages) => {
                self.tag(b"messages");
                for message in messages {
                    self.visit_message(message);
                }
            }
            Payload::WorkerSet(worker_set) => {
                self.tag(b"worker_set");
                self.visit_worker_set(worker_set)
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

    fn visit_worker_set(&mut self, worker_set: &WorkerSet) {
        for signer in worker_set.signers.values() {
            self.visit_signer(signer);
        }
        self.visit_u256(&worker_set.threshold);
        self.visit_u64(&worker_set.created_at)
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

    fn visit_u64(&mut self, number: &u64);
    fn visit_u256(&mut self, number: &U256);
    fn visit_bytes(&mut self, bytes: &[u8]);

    /// No-op by default.
    fn tag(&mut self, _bytes: &[u8]) {}
}
