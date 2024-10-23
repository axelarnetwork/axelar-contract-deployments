//! Module for the `GatewayExecuteData` account type.
use std::borrow::Cow;
use std::fmt::Debug;

use axelar_rkyv_encoding::hasher::AxelarRkyv256Hasher;
use axelar_rkyv_encoding::rkyv::bytecheck::{self, CheckBytes, StructCheckError};
use axelar_rkyv_encoding::rkyv::ser::serializers::{
    AlignedSerializer, AllocScratch, CompositeSerializer, FallbackScratch, HeapScratch,
    SharedSerializeMap,
};
use axelar_rkyv_encoding::types::{
    ArchivedHasheableMessageVec, ArchivedVerifierSet, ExecuteData, HasheableMessageVec, Payload,
    Proof, VerifierSet,
};
use axelar_rkyv_encoding::visitor::Visitor;
use rkyv::validation::validators::{DefaultValidator, DefaultValidatorError};
use rkyv::{AlignedVec, Archive, Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;
use crate::hasher_impl;
use crate::processor::ToBytes;

/// Hash of the data the within the ExecuteData + the proof.
pub type ExecuteDataHash = [u8; 32];

/// Data for the `ApproveMessages` command.
pub type ApproveMessagesVariant = HasheableMessageVec;

/// Data for the `RotateSigners` command.
pub type RotateSignersVariant = VerifierSet;

/// Trait required for types that can be used as a variant for data related
/// to a command.
pub trait ExecuteDataVariant:
    Archive<Archived = Self::ArchivedData>
    + TryFrom<Payload>
    + Visitable
    + PartialEq
    + Eq
    + Serialize<
        CompositeSerializer<
            AlignedSerializer<AlignedVec>,
            FallbackScratch<HeapScratch<0>, AllocScratch>,
            SharedSerializeMap,
        >,
    > + Debug
{
    /// The archived version of the data.
    type ArchivedData: Debug + PartialEq + Eq;
}

impl ExecuteDataVariant for ApproveMessagesVariant {
    type ArchivedData = ArchivedHasheableMessageVec;
}
impl ExecuteDataVariant for RotateSignersVariant {
    type ArchivedData = ArchivedVerifierSet;
}

/// Gateway Execute Data type.
/// Represents the execution data for an ApproveMessages or a RotateSigners
/// gateway transaction.
#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct GatewayExecuteData<T>
where
    T: ExecuteDataVariant,
{
    /// The list of messages to verify or the new verifier set
    pub data: T,

    /// The proof signed by the Axelar signers for this command.
    pub proof: Proof,

    /// Pre-computed message payload hash
    pub payload_hash: [u8; 32],

    /// The bump seed for the PDA account.
    pub bump: u8,
}

impl<T> ToBytes for GatewayExecuteData<T>
where
    T: ExecuteDataVariant,
{
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, GatewayError> {
        let bytes: Vec<u8> = rkyv::to_bytes::<_, 0>(self)
            .map_err(|_| GatewayError::ByteSerializationError)?
            .to_vec();

        Ok(Cow::Owned(bytes))
    }
}

impl<T> GatewayExecuteData<T>
where
    T: ExecuteDataVariant,
{
    /// Creates a new `GatewayExecuteData` struct.
    pub fn new(
        data: &[u8],
        gateway_root_pda: &Pubkey,
        domain_separator: &[u8; 32],
    ) -> Result<GatewayExecuteData<T>, GatewayError> {
        let Ok(execute_data) = ExecuteData::from_bytes(data) else {
            return Err(GatewayError::MalformedProof);
        };

        let payload_hash = axelar_rkyv_encoding::hash_payload(
            domain_separator,
            &execute_data.proof.verifier_set(*domain_separator),
            &execute_data.payload,
            hasher_impl(),
        );

        let mut gateway_execute_data = Self {
            data: execute_data
                .payload
                .try_into()
                .map_err(|_| GatewayError::ByteSerializationError)?,
            proof: execute_data.proof,
            payload_hash,
            bump: 0, // bump will be set after we derive the PDA
        };

        let hash = gateway_execute_data.hash_decoded_contents();
        let (_pubkey, bump) = crate::get_execute_data_pda(gateway_root_pda, &hash);
        gateway_execute_data.bump = bump;

        Ok(gateway_execute_data)
    }

    /// Returns hash of the contents from the decoded ExecuteData.
    pub fn hash_decoded_contents(&self) -> [u8; 32] {
        let mut hasher = hasher_impl();
        self.data.accept(&mut hasher);
        Visitor::visit_proof(&mut hasher, &self.proof);
        hasher.result().into()
    }
}

impl<'a, T> ArchivedGatewayExecuteData<T>
where
    T: ExecuteDataVariant,
    T::ArchivedData: CheckBytes<DefaultValidator<'a>>,
{
    /// Tries to interpret bytes as an archived ApproveMessagesExecuteData.
    pub fn from_bytes(
        bytes: &'a [u8],
    ) -> Result<&Self, rkyv::validation::CheckArchiveError<StructCheckError, DefaultValidatorError>>
    {
        rkyv::check_archived_root::<GatewayExecuteData<T>>(bytes)
    }
}

/// Trait for types that can be visited by a `Visitor`.
pub trait Visitable {
    /// Accepts a visitor to visit the implementing type.
    fn accept<'a, V: Visitor<'a>>(&'a self, visitor: &mut V);
}

impl Visitable for ApproveMessagesVariant {
    fn accept<'a, V: Visitor<'a>>(&'a self, visitor: &mut V) {
        visitor.visit_messages(self)
    }
}

impl Visitable for RotateSignersVariant {
    fn accept<'a, V: Visitor<'a>>(&'a self, visitor: &mut V) {
        visitor.visit_verifier_set(self)
    }
}

#[test]
fn test_gateway_approve_messages_execute_data_roundtrip() {
    use axelar_rkyv_encoding::test_fixtures::{
        random_messages, random_valid_execute_data_and_verifier_set_for_payload,
    };
    use axelar_rkyv_encoding::types::{HasheableMessageVec, Payload};

    let domain_separator = [5; 32];
    let gateway_root_pda = Pubkey::new_unique();
    let payload = Payload::new_messages(random_messages());
    let (execute_data, _) =
        random_valid_execute_data_and_verifier_set_for_payload(domain_separator, payload);
    let raw_data = execute_data.to_bytes::<0>().unwrap();

    let gateway_execute_data = GatewayExecuteData::<HasheableMessageVec>::new(
        &raw_data,
        &gateway_root_pda,
        &domain_separator,
    )
    .unwrap();
    let serialized_gateway_execute_data = ToBytes::to_bytes(&gateway_execute_data).unwrap();

    let execute_data_pda = ArchivedGatewayExecuteData::<HasheableMessageVec>::from_bytes(
        &serialized_gateway_execute_data[..],
    )
    .unwrap();

    let original_messages: HasheableMessageVec = execute_data.payload.try_into().unwrap();

    assert_eq!(execute_data_pda.proof, execute_data.proof);
    assert_eq!(execute_data_pda.data, original_messages);
}

#[test]
fn test_gateway_rotate_signers_execute_data_roundtrip() {
    use axelar_rkyv_encoding::test_fixtures::{
        random_valid_execute_data_and_verifier_set_for_payload, random_valid_verifier_set,
    };
    use axelar_rkyv_encoding::types::{Payload, VerifierSet};

    let domain_separator = [5; 32];
    let gateway_root_pda = Pubkey::new_unique();
    let payload = Payload::new_verifier_set(random_valid_verifier_set());
    let (execute_data, _) =
        random_valid_execute_data_and_verifier_set_for_payload(domain_separator, payload);
    let raw_data = execute_data.to_bytes::<0>().unwrap();

    let gateway_execute_data =
        GatewayExecuteData::<VerifierSet>::new(&raw_data, &gateway_root_pda, &domain_separator)
            .unwrap();
    let serialized_gateway_execute_data = ToBytes::to_bytes(&gateway_execute_data).unwrap();

    let execute_data_pda =
        ArchivedGatewayExecuteData::<VerifierSet>::from_bytes(&serialized_gateway_execute_data[..])
            .unwrap();

    let original_set: VerifierSet = execute_data.payload.try_into().unwrap();

    assert_eq!(execute_data_pda.proof, execute_data.proof);
    assert_eq!(execute_data_pda.data, original_set);
}
