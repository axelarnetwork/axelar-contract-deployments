use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleProof, SolanaSyscallHasher};
use axelar_rkyv_encoding::types::{PublicKey, Signature, VerifierSetLeafNode};
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use static_assertions::assert_eq_align;

use super::Processor;
use crate::state::signature_verification::SignatureVerifier;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::state::GatewayConfig;

/// This a buffer array needs to have the same size and alignment as the
/// signature verification session PDA, otherwise [`bytemuck`] will fail to cast
/// the account data as our type.
#[repr(C, align(16))]
struct Aligned16 {
    data: [u8; SignatureVerificationSessionData::LEN],
}
assert_eq_align!(SignatureVerificationSessionData, Aligned16);

impl Processor {
    /// Handles the
    /// [`crate::instructions::GatewayInstruction::InitializePayloadVerificationSession`]
    /// instruction.
    pub fn process_verify_signature(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        payload_merkle_root: [u8; 32],
        verifier_set_leaf_node: VerifierSetLeafNode<SolanaSyscallHasher>,
        verifier_merkle_proof: MerkleProof<SolanaSyscallHasher>,
        signature: Signature,
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let verifier_set_tracker_account = next_account_info(accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        let gateway_config = gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        // Access signature verification session data
        if verification_session_account.data_len() != SignatureVerificationSessionData::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut buffer = Aligned16 {
            data: [0u8; SignatureVerificationSessionData::LEN],
        };
        let mut data = verification_session_account.try_borrow_mut_data()?;
        buffer.data.copy_from_slice(&data);
        let session: &mut SignatureVerificationSessionData = bytemuck::cast_mut(&mut buffer.data);

        // Check: Verification PDA can be derived from seeds stored into the account
        // data itself.
        {
            let expected_pda = crate::create_signature_verification_pda(
                gateway_root_pda.key,
                &payload_merkle_root,
                session.bump,
            )?;
            if expected_pda != *verification_session_account.key {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // Obtain the active verifier set tracker.
        let verifier_set_tracker = verifier_set_tracker_account
            .check_initialized_pda::<VerifierSetTracker>(program_id)
            .map_err(|error| {
                solana_program::msg!("Invalid VerifierSetTracker PDA");
                error
            })?;

        // Check: Verifier set isn't expired
        let is_epoch_valid = gateway_config
            .is_epoch_valid(verifier_set_tracker.epoch)
            .map_err(|err| {
                solana_program::msg!("AuthWeightedError: {}", err);
                ProgramError::InvalidInstructionData
            })?;
        if !is_epoch_valid {
            solana_program::msg!("Expired VerifierSetTracker PDA");
            return Err(ProgramError::InvalidAccountData);
        }

        // Verify the signature
        session
            .signature_verification
            .process_signature(
                verifier_set_leaf_node,
                &verifier_merkle_proof,
                &verifier_set_tracker.verifier_set_hash,
                &payload_merkle_root,
                &signature,
                &(GatewaySignatureVerifier {}),
            )
            .map_err(|error| {
                solana_program::msg!("Error: {}", error);
                ProgramError::InvalidInstructionData
            })?;

        // Write the bytes back into account data
        data.copy_from_slice(&buffer.data);

        Ok(())
    }
}

struct GatewaySignatureVerifier {}
impl SignatureVerifier for GatewaySignatureVerifier {
    fn verify_signature(
        &self,
        _signature: &Signature,
        _public_key: &PublicKey,
        _message: &[u8; 32],
    ) -> bool {
        // WARN: This will always verify the signature without looking at the inputs
        // TODO: implement this.
        true
    }
}
