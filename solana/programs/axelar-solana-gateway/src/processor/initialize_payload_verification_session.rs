use std::mem::size_of;

use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::{BytemuckedPda, GatewayConfig};
use crate::{assert_valid_gateway_root_pda, seed_prefixes};

impl Processor {
    /// Handles the
    /// [`crate::instructions::GatewayInstruction::InitializePayloadVerificationSession`]
    /// instruction.
    pub fn process_initialize_payload_verification_session(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        merkle_root: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        assert!(payer.is_signer);
        assert!(payer.is_writable);
        assert!(!verification_session_account.is_signer);
        assert!(verification_session_account.is_writable);
        assert_eq!(verification_session_account.lamports(), 0);
        assert!(system_program::check_id(system_program.key));

        // Check: Gateway Root PDA is initialized.
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = gateway_root_pda.try_borrow_data()?;
        let gateway_config = GatewayConfig::read(&data)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: Verification PDA can be derived from provided seeds.
        // using canonical bump for the session account
        let (verification_session_pda, bump) =
            crate::get_signature_verification_pda(gateway_root_pda.key, &merkle_root);
        if verification_session_pda != *verification_session_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Use the same seeds as `[crate::get_signature_verification_pda]`, plus the
        // bump seed.
        let signers_seeds = &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            gateway_root_pda.key.as_ref(),
            &merkle_root,
            &[bump],
        ];

        // Prepare the `create_account` instruction
        program_utils::init_pda_raw(
            payer,
            verification_session_account,
            program_id,
            system_program,
            size_of::<SignatureVerificationSessionData>() as u64,
            signers_seeds,
        )?;
        let mut data = verification_session_account.try_borrow_mut_data()?;
        let session = SignatureVerificationSessionData::read_mut(&mut data)?;
        session.bump = bump;

        Ok(())
    }
}
