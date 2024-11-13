use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{system_instruction, system_program};

use super::Processor;
use crate::seed_prefixes;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::GatewayConfig;

impl Processor {
    /// Handles the
    /// [`crate::instructions::GatewayInstruction::InitializePayloadVerificationSession`]
    /// instruction.
    pub fn process_initialize_payload_verification_session(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        merkle_root: [u8; 32],
        bump: u8,
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
        gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        // Check: Verification PDA can be derived from provided seeds.
        let verification_session_pda =
            crate::create_signature_verification_pda(gateway_root_pda.key, &merkle_root, bump)?;
        if verification_session_pda != *verification_session_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Prepare the `create_account` instruction
        let lamports_required = Rent::get()?.minimum_balance(SignatureVerificationSessionData::LEN);
        let create_pda_account_ix = system_instruction::create_account(
            payer.key,
            verification_session_account.key,
            lamports_required,
            SignatureVerificationSessionData::LEN as u64,
            program_id,
        );

        // Use the same seeds as `[crate::get_signature_verification_pda]`, plus the
        // bump seed.
        let signers_seeds = &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            gateway_root_pda.key.as_ref(),
            &merkle_root,
            &[bump],
        ];

        // Create the empty verification account.
        invoke_signed(
            &create_pda_account_ix,
            &[
                payer.clone(),
                verification_session_account.clone(),
                system_program.clone(),
            ],
            &[signers_seeds],
        )?;

        // Set the account data
        let mut data = verification_session_account.try_borrow_mut_data()?;
        let data_bytes: &mut [u8; SignatureVerificationSessionData::LEN] =
            (*data).try_into().map_err(|_err| {
                solana_program::msg!("session account data is corrupt");
                ProgramError::InvalidAccountData
            })?;
        let session: &mut SignatureVerificationSessionData = bytemuck::cast_mut(data_bytes);
        session.bump = bump;

        Ok(())
    }
}
