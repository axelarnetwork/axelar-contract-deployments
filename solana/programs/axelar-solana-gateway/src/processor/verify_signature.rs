
use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleProof, SolanaSyscallHasher};
use axelar_rkyv_encoding::types::{PublicKey, Signature, VerifierSetLeafNode};
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::axelar_auth_weighted::verify_ecdsa_signature;
use crate::state::signature_verification::SignatureVerifier;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::state::GatewayConfig;

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

        let mut data = verification_session_account.try_borrow_mut_data()?;
        let data_bytes: &mut [u8; SignatureVerificationSessionData::LEN] =
            (*data).try_into().map_err(|_err| {
                solana_program::msg!("session account data is corrupt");
                ProgramError::InvalidAccountData
            })?;
        let session: &mut SignatureVerificationSessionData = bytemuck::cast_mut(data_bytes);

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
                &OnChainSignatureVerifier,
            )
            .map_err(|error| {
                solana_program::msg!("Error: {}", error);
                ProgramError::InvalidInstructionData
            })?;

        Ok(())
    }
}

/// Performs elliptic curve calculations on chain to verify digital signatures.
struct OnChainSignatureVerifier;

impl SignatureVerifier for OnChainSignatureVerifier {
    fn verify_signature(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        message: &[u8; 32],
    ) -> bool {
        match (signature, public_key) {
            (Signature::EcdsaRecoverable(signature), PublicKey::Secp256k1(pubkey)) => {
                verify_ecdsa_signature(pubkey, signature, message)
            }
            (Signature::Ed25519(_), PublicKey::Ed25519(_)) => {
                unimplemented!("ed25519 signature verification is not implemented")
            }
            _ => {
                solana_program::msg!(
                    "Error: Invalid combination of Secp256k1 and Ed25519 signature and public key"
                );
                false
            }
        }
    }
}
