use std::collections::BTreeMap;

use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Deserialize, Fallible, Serialize};

const ED25519_PUBKEY_LEN: usize = 32;
const ECDSA_COMPRESSED_PUBKEY_LEN: usize = 33;

const ED25519_SIGNATURE_LEN: usize = 64;
const ECDSA_RECOVERABLE_SIGNATURE_LEN: usize = 65;

type EcdsaPubkey = [u8; ECDSA_COMPRESSED_PUBKEY_LEN];
type Ed25519Pubkey = [u8; ED25519_PUBKEY_LEN];

type EcdsaRecoverableSignature = [u8; ECDSA_RECOVERABLE_SIGNATURE_LEN];
type Ed25519Signature = [u8; ED25519_SIGNATURE_LEN];

#[derive(Debug, thiserror::Error)]
pub enum EncodingError<const N: usize> {
    #[error("Serialization error")]
    Serialize(#[source] <AllocSerializer<N> as Fallible>::Error),
}

#[derive(Clone, Copy, Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct U256([u8; 32]);

impl U256 {
    pub fn from_be(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn to_be(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Archive, Deserialize, Serialize, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Debug)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum PublicKey {
    Ecdsa(EcdsaPubkey),
    Ed25519(Ed25519Pubkey),
}

impl PublicKey {
    pub fn new_ecdsa(pubkey: EcdsaPubkey) -> Self {
        PublicKey::Ecdsa(pubkey)
    }

    pub fn new_ed25519(pubkey: Ed25519Pubkey) -> Self {
        PublicKey::Ed25519(pubkey)
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum Signature {
    EcdsaRecoverable(EcdsaRecoverableSignature),
    Ed25519(Ed25519Signature),
}

impl Signature {
    pub fn new_ecdsa_recoverable(bytes: EcdsaRecoverableSignature) -> Self {
        Self::EcdsaRecoverable(bytes)
    }

    pub fn new_ed25519(bytes: Ed25519Signature) -> Self {
        Self::Ed25519(bytes)
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct CrossChainId {
    pub(crate) chain: String,
    pub(crate) id: String,
}

impl CrossChainId {
    pub fn new(chain: String, id: String) -> Self {
        Self { chain, id }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct Message {
    pub(crate) cc_id: CrossChainId,
    pub(crate) source_address: String,
    pub(crate) destination_chain: String,
    pub(crate) destination_address: String,
    pub(crate) payload_hash: [u8; 32],
}

impl Message {
    pub fn new(
        cc_id: CrossChainId,
        source_address: String,
        destination_chain: String,
        destination_address: String,
        payload_hash: [u8; 32],
    ) -> Self {
        Self {
            cc_id,
            source_address,
            destination_chain,
            destination_address,
            payload_hash,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct Signer {
    pub(crate) address: String,
    pub(crate) public_key: PublicKey,
    pub(crate) weight: U256,
}

impl Signer {
    pub fn new(address: String, public_key: PublicKey, weight: U256) -> Self {
        Self {
            address,
            public_key,
            weight,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct WorkerSet {
    pub(crate) created_at: u64,
    pub(crate) signers: BTreeMap<String, Signer>,
    pub(crate) threshold: U256,
}

impl WorkerSet {
    pub fn new(created_at: u64, signers: BTreeMap<String, Signer>, threshold: U256) -> Self {
        Self {
            created_at,
            signers,
            threshold,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum Payload {
    Messages(Vec<Message>),
    WorkerSet(WorkerSet),
}

impl Payload {
    pub fn new_messages(messages: Vec<Message>) -> Self {
        Self::Messages(messages)
    }

    pub fn new_worker_set(worker_set: WorkerSet) -> Self {
        Self::WorkerSet(worker_set)
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct WeightedSignature {
    pub(crate) pubkey: PublicKey,
    pub(crate) signature: Signature,
    pub(crate) weight: U256,
}

impl WeightedSignature {
    pub fn new(pubkey: PublicKey, signature: Signature, weight: U256) -> Self {
        Self {
            pubkey,
            signature,
            weight,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub(crate) struct Proof {
    signatures: Vec<WeightedSignature>,
    threshold: U256,
    nonce: u64,
}

impl Proof {
    pub(crate) fn new(signatures: Vec<WeightedSignature>, threshold: U256, nonce: u64) -> Self {
        Self {
            signatures,
            threshold,
            nonce,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub(crate) struct ExecuteData {
    payload: Payload,
    proof: Proof,
}

impl ExecuteData {
    pub(crate) fn new(payload: Payload, proof: Proof) -> Self {
        Self { payload, proof }
    }
}
