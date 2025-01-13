use core::mem::size_of;
use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::error::GatewayError;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::GatewayConfig;
use crate::{assert_valid_gateway_root_pda, seed_prefixes};

impl Processor {
    /// Initializes a signature verification session PDA account for a given Axelar payload (former
    /// `execute_data`).
    ///
    /// Creates a [`SignatureVerificationSession`] PDA account to track signature verification state
    /// for a batch of messages identified by the Merkle root of the Axelar payload.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Required accounts are missing or in wrong order.
    /// * Account permissions are invalid.
    /// * System program account is invalid.
    ///
    /// Returns [`GatewayError`] if:
    /// * Gateway root PDA is not initialized or invalid.
    /// * Verification session PDA derivation fails.
    /// * Session account is already initialized.
    /// * Data serialization fails.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * Converting `size_of::<SignatureVerificationSessionData>` to `u64` overflows (via `expect`, unlikely to happen)
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

        // Check payer account requirements
        if !payer.is_signer {
            solana_program::msg!("Error: payer account is not a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_writable {
            solana_program::msg!("Error: payer account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check verification session account requirements
        if verification_session_account.is_signer {
            solana_program::msg!("Error: verification session account is not a signer");
            return Err(ProgramError::InvalidAccountData);
        }
        if !verification_session_account.is_writable {
            solana_program::msg!("Error: verification session account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if verification_session_account.lamports() != 0 {
            solana_program::msg!("Error: verification session account is not initialized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // Check system program
        if !system_program::check_id(system_program.key) {
            solana_program::msg!("Error: invalid system program account");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: Gateway Root PDA is initialized.
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = gateway_root_pda.try_borrow_data()?;
        let gateway_config =
            GatewayConfig::read(&data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: Verification PDA can be derived from provided seeds.
        // using canonical bump for the session account
        let (verification_session_pda, bump) =
            crate::get_signature_verification_pda(gateway_root_pda.key, &merkle_root);
        if verification_session_pda != *verification_session_account.key {
            return Err(GatewayError::InvalidVerificationSessionPDA.into());
        }

        // Check: the verification session account has not been initialised already
        verification_session_account
            .check_uninitialized_pda()
            .map_err(|_err| GatewayError::VerificationSessionPDAInitialised)?;

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
            size_of::<SignatureVerificationSessionData>()
                .try_into()
                .map_err(|_err| {
                    solana_program::msg!("Unexpected u64 overflow in struct size");
                    ProgramError::ArithmeticOverflow
                })?,
            signers_seeds,
        )?;
        let mut data = verification_session_account.try_borrow_mut_data()?;
        let session = SignatureVerificationSessionData::read_mut(&mut data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        session.bump = bump;

        Ok(())
    }
}
