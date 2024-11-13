//! Contains proxy types to gap the bridge between rkyv and borsh serialization
//! frameworks.
//!
//! The types declared in this module mirror the data layout of their
//! counterparts of wrapping the original type because by doing so we save one
//! round of deserialization on-chain.
//!
//! Also, the [`Merkle`] trait is implemented for the "unarchived" types,
//! meaning we don't benefit of rkyv's "zero-copy" deserialization.
//!
//! Once we move to a single serialization framework we won't need this module
//! anymore.

use axelar_rkyv_encoding::hasher::merkle_tree::SolanaSyscallHasher;
use axelar_rkyv_encoding::types::{
    CrossChainId, EcdsaRecoverableSignature, Ed25519Pubkey, Ed25519Signature, MessageElement, PublicKey, Secp256k1Pubkey, Signature, VerifierSetElement,
    VerifierSetLeafNode,
};
use borsh::{BorshDeserialize, BorshSerialize};

/// Proxy for [`VerifierSetLeafNode<SolanaSyscalHasher>`].
///
/// Necessary because internal types don't implement the required traits (mostly
/// Borsh's) to be used as Gateway' instruction parameters.
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct ProxyMessageLeafNode {
    chain: String,
    id: String,
    source_address: String,
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    domain_separator: [u8; 32],
    position: u16,
    set_size: u16,
}

impl From<MessageElement> for ProxyMessageLeafNode {
    fn from(leaf: MessageElement) -> Self {
        let MessageElement {
            message,
            position,
            num_messages,
        } = leaf;
        Self {
            chain: message.cc_id.chain,
            id: message.cc_id.id,
            source_address: message.source_address,
            destination_chain: message.destination_chain,
            destination_address: message.destination_address,
            payload_hash: message.payload_hash,
            domain_separator: message.domain_separator,
            position,
            set_size: num_messages,
        }
    }
}

impl From<ProxyMessageLeafNode> for MessageElement {
    fn from(leaf: ProxyMessageLeafNode) -> Self {
        let ProxyMessageLeafNode {
            chain,
            id,
            source_address,
            destination_chain,
            destination_address,
            payload_hash,
            domain_separator,
            position,
            set_size,
        } = leaf;
        Self {
            message: axelar_rkyv_encoding::types::Message {
                cc_id: CrossChainId::new(chain, id),
                source_address,
                destination_chain,
                destination_address,
                payload_hash,
                domain_separator,
            },
            position,
            num_messages: set_size,
        }
    }
}

/// Proxy for [`VerifierSetLeafNode<SolanaSyscalHasher>`].
///
/// Necessary because internal types don't implement the required traits (mostly
/// Borsh's) to be used as Gateway' instruction parameters.
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct ProxyVerifierSetLeafNode {
    created_at: u64,
    quorum: u128,
    signer_pubkey: ProxyPublicKey,
    signer_weight: u128,
    domain_separator: [u8; 32],
    position: u16,
    set_size: u16,
}

impl From<VerifierSetLeafNode<SolanaSyscallHasher>> for ProxyVerifierSetLeafNode {
    fn from(leaf: VerifierSetLeafNode<SolanaSyscallHasher>) -> Self {
        let VerifierSetElement {
            created_at,
            quorum,
            signer_pubkey,
            signer_weight,
            domain_separator,
            position,
            set_size,
        } = *leaf;
        Self {
            created_at,
            quorum,
            signer_pubkey: signer_pubkey.into(),
            signer_weight,
            domain_separator,
            position,
            set_size,
        }
    }
}

impl From<ProxyVerifierSetLeafNode> for VerifierSetLeafNode<SolanaSyscallHasher> {
    fn from(proxy: ProxyVerifierSetLeafNode) -> Self {
        let ProxyVerifierSetLeafNode {
            created_at,
            quorum,
            signer_pubkey,
            signer_weight,
            domain_separator,
            position,
            set_size,
        } = proxy;
        VerifierSetElement {
            created_at,
            quorum,
            signer_pubkey: signer_pubkey.into(),
            signer_weight,
            domain_separator,
            position,
            set_size,
        }
        .into()
    }
}

#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
enum ProxyPublicKey {
    Secp256k1(Secp256k1Pubkey),
    Ed25519(Ed25519Pubkey),
}

impl From<PublicKey> for ProxyPublicKey {
    fn from(pubkey: PublicKey) -> Self {
        match pubkey {
            PublicKey::Secp256k1(pk) => Self::Secp256k1(pk),
            PublicKey::Ed25519(pk) => Self::Ed25519(pk),
        }
    }
}

impl From<ProxyPublicKey> for PublicKey {
    fn from(proxy: ProxyPublicKey) -> Self {
        match proxy {
            ProxyPublicKey::Secp256k1(pk) => Self::Secp256k1(pk),
            ProxyPublicKey::Ed25519(pk) => Self::Ed25519(pk),
        }
    }
}

/// Proxy for [`Signature`].
///
/// Necessary because internal types don't implement the required traits (mostly
/// Borsh's) to be used as Gateway' instruction parameters.
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum ProxySignature {
    EcdsaRecoverable(EcdsaRecoverableSignature),
    Ed25519(Ed25519Signature),
}

impl From<Signature> for ProxySignature {
    fn from(signature: Signature) -> Self {
        match signature {
            Signature::EcdsaRecoverable(sig) => Self::EcdsaRecoverable(sig),
            Signature::Ed25519(sig) => Self::Ed25519(sig),
        }
    }
}

impl From<ProxySignature> for Signature {
    fn from(proxy: ProxySignature) -> Self {
        match proxy {
            ProxySignature::EcdsaRecoverable(sig) => Self::EcdsaRecoverable(sig),
            ProxySignature::Ed25519(sig) => Self::Ed25519(sig),
        }
    }
}

#[cfg(test)]
mod tests {
    use axelar_rkyv_encoding::hasher::merkle_trait::Merkle;
    use axelar_rkyv_encoding::test_fixtures::random_valid_verifier_set;

    use super::*;

    #[test]
    fn test_serialization() {
        let verifier_set = random_valid_verifier_set();
        assert!(!verifier_set.signers().is_empty());
        for leaf in Merkle::<SolanaSyscallHasher>::merkle_leaves(&verifier_set) {
            let wrapper: ProxyVerifierSetLeafNode = leaf.into();
            let serialized = borsh::to_vec(&wrapper).unwrap();
            let deserialized = borsh::from_slice(&serialized).unwrap();
            assert_eq!(wrapper, deserialized);
            let de_leaf: VerifierSetLeafNode<SolanaSyscallHasher> = wrapper.into();
            assert_eq!(*de_leaf, *leaf);
        }
    }
}
