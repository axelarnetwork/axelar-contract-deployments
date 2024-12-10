use axelar_solana_encoding::types::execute_data::SigningVerifierSetInfo;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::state::{BytemuckedPda, GatewayConfig};
use crate::{
    assert_valid_gateway_root_pda, assert_valid_signature_verification_pda,
    assert_valid_verifier_set_tracker_pda,
};

impl Processor {
    /// Handles the
    /// [`crate::instructions::GatewayInstruction::InitializePayloadVerificationSession`]
    /// instruction.
    pub fn process_verify_signature(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        payload_merkle_root: [u8; 32],
        verifier_info: SigningVerifierSetInfo,
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let verifier_set_tracker_account = next_account_info(accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = gateway_root_pda.try_borrow_data()?;
        let gateway_config = GatewayConfig::read(&data)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: Verification session PDA is initialized.
        verification_session_account.check_initialized_pda_without_deserialization(program_id)?;
        let mut data = verification_session_account.try_borrow_mut_data()?;
        let session = SignatureVerificationSessionData::read_mut(&mut data)?;
        assert_valid_signature_verification_pda(
            gateway_root_pda.key,
            &payload_merkle_root,
            session.bump,
            verification_session_account.key,
        )?;

        // Check: Active verifier set tracker PDA is initialized.
        verifier_set_tracker_account.check_initialized_pda_without_deserialization(program_id)?;
        let data = verifier_set_tracker_account.try_borrow_data()?;
        let verifier_set_tracker = VerifierSetTracker::read(&data)?;
        assert_valid_verifier_set_tracker_pda(
            verifier_set_tracker,
            verifier_set_tracker_account.key,
        )?;

        // Check: Verifier set isn't expired
        gateway_config.assert_valid_epoch(verifier_set_tracker.epoch)?;

        // Verify the signature
        session
            .signature_verification
            .process_signature(
                verifier_info,
                &verifier_set_tracker.verifier_set_hash,
                &payload_merkle_root,
            )
            .map_err(|error| {
                solana_program::msg!("Error: {}", error);
                ProgramError::InvalidInstructionData
            })?;

        Ok(())
    }
}
