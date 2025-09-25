use core::mem::size_of;

use axelar_message_primitives::U256;
use program_utils::{
    pda::{BytemuckedPda, ValidPDA},
    validate_system_account_key,
};
use role_management::processor::ensure_upgrade_authority;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::sysvar::Sysvar;

use super::Processor;
use crate::error::GatewayError;
use crate::instructions::InitializeConfig;
use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::state::GatewayConfig;
use crate::{
    assert_valid_gateway_root_pda, assert_valid_verifier_set_tracker_pda,
    get_gateway_root_config_pda, get_verifier_set_tracker_pda, seed_prefixes,
};

impl Processor {
    /// Initializes the gateway program by setting up configuration and a verifier set account.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Required accounts are missing or in wrong order
    /// * Upgrade authority validation fails
    /// * System program account is invalid
    /// * PDA derivations fails
    /// * Account initialization fails
    ///
    /// Returns [`GatewayError`] if:
    /// * Data serialization/deserialization fails
    /// * Invalid PDA bumps are provided
    ///
    /// # Security Considerations
    ///
    /// * Only the program upgrade authority can call this instruction.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * Converting `size_of::<VerifierSetTracker>` to u64 overflows (via `expect`)
    /// * Converting `size_of::<GatewayConfig>` to u64 overflows (via `expect`)
    /// * Converting `unix_timestamp` to u64 results in an invalid timestamp (via `expect`)
    pub fn process_initialize_config(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        init_config: &InitializeConfig,
    ) -> ProgramResult {
        let accounts = &mut accounts.iter();
        let payer = next_account_info(accounts)?;
        let upgrade_authority = next_account_info(accounts)?;
        let program_data = next_account_info(accounts)?;
        let gateway_root_pda = next_account_info(accounts)?;
        let system_account = next_account_info(accounts)?;
        let verifier_set_pda = next_account_info(accounts)?;

        validate_system_account_key(system_account.key)?;

        // Check: Upgrade authority
        ensure_upgrade_authority(program_id, upgrade_authority, program_data)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Initialize single verifier set
        let verifier_set_hash = init_config.initial_verifier_set.hash;
        let epoch = U256::from_u64(1);
        let current_epochs = U256::from_u64(1);

        let (_, pda_bump) = get_verifier_set_tracker_pda(verifier_set_hash);
        verifier_set_pda.check_uninitialized_pda()?;

        // Initialize the tracker account
        program_utils::pda::init_pda_raw(
            payer,
            verifier_set_pda,
            program_id,
            system_account,
            size_of::<VerifierSetTracker>().try_into().map_err(|_err| {
                solana_program::msg!("unexpected u64 overflow in struct size");
                ProgramError::ArithmeticOverflow
            })?,
            &[
                seed_prefixes::VERIFIER_SET_TRACKER_SEED,
                verifier_set_hash.as_slice(),
                &[pda_bump],
            ],
        )?;

        // store account data
        let mut data = verifier_set_pda.try_borrow_mut_data()?;
        let tracker =
            VerifierSetTracker::read_mut(&mut data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        *tracker = VerifierSetTracker::new(pda_bump, epoch, verifier_set_hash);

        // check that everything has been derived correctly
        assert_valid_verifier_set_tracker_pda(tracker, verifier_set_pda.key)?;

        let (_, bump) = get_gateway_root_config_pda();

        // Check: Gateway Config account uses the canonical bump.
        assert_valid_gateway_root_pda(bump, gateway_root_pda.key)?;

        // Initialize the account
        program_utils::pda::init_pda_raw(
            payer,
            gateway_root_pda,
            program_id,
            system_account,
            size_of::<GatewayConfig>().try_into().map_err(|_err| {
                solana_program::msg!("unexpected u64 overflow in struct size");
                ProgramError::ArithmeticOverflow
            })?,
            &[seed_prefixes::GATEWAY_SEED, &[bump]],
        )?;
        let mut data = gateway_root_pda.try_borrow_mut_data()?;
        let gateway_config =
            GatewayConfig::read_mut(&mut data).ok_or(GatewayError::BytemuckDataLenInvalid)?;

        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp.try_into().map_err(|_err| {
            solana_program::msg!("invalid timestamp");
            ProgramError::ArithmeticOverflow
        })?;
        *gateway_config = GatewayConfig::new(
            current_epochs,
            init_config.previous_verifier_retention,
            init_config.minimum_rotation_delay,
            current_timestamp,
            init_config.operator,
            init_config.domain_separator,
            bump,
        );

        Ok(())
    }
}
