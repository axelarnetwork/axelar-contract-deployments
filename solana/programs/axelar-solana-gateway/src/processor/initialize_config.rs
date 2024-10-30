use axelar_message_primitives::U256;
use itertools::Itertools;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_program};

use super::Processor;
use crate::axelar_auth_weighted::AxelarAuthWeighted;
use crate::error::GatewayError;
use crate::instructions::InitializeConfig;
use crate::state::verifier_set_tracker::{VerifierSetHash, VerifierSetTracker};
use crate::state::GatewayConfig;
use crate::{
    assert_valid_gateway_root_pda, assert_valid_verifier_set_tracker_pda,
    get_gateway_root_config_internal, seed_prefixes,
};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_config(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        init_config: InitializeConfig<(VerifierSetHash, u8)>,
    ) -> ProgramResult {
        let (core_accounts, init_verifier_sets) = accounts.split_at(3);

        let init_verifier_sets = &mut init_verifier_sets.iter();
        let core_accounts = &mut core_accounts.iter();
        let payer = next_account_info(core_accounts)?;
        let gateway_root_pda = next_account_info(core_accounts)?;
        let system_account = next_account_info(core_accounts)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }
        let verifier_sets = init_config
            .initial_signer_sets
            .iter()
            .zip_eq(init_verifier_sets);
        let current_epochs: u64 = verifier_sets.len().try_into().unwrap();
        let current_epochs = U256::from_u64(current_epochs);

        for (idx, ((verifier_set_hash, pda_bump), verifier_set_pda)) in verifier_sets.enumerate() {
            let idx: u64 = idx.try_into().map_err(|_| {
                msg!("could not transform idx");
                ProgramError::InvalidInstructionData
            })?;
            let epoch = U256::from_u64(idx + 1);
            let tracker = VerifierSetTracker {
                bump: *pda_bump,
                epoch,
                verifier_set_hash: *verifier_set_hash,
            };
            // check that everything has been derived correctly
            assert_valid_verifier_set_tracker_pda(&tracker, verifier_set_pda.key);
            verifier_set_pda.check_uninitialized_pda()?;
            program_utils::init_pda(
                payer,
                verifier_set_pda,
                program_id,
                system_account,
                tracker,
                &[
                    seed_prefixes::VERIFIER_SET_TRACKER_SEED,
                    verifier_set_hash.as_slice(),
                    &[*pda_bump],
                ],
            )?;
        }
        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp.try_into().expect("invalid timestamp");
        let auth_weighted = AxelarAuthWeighted::new(
            init_config.previous_signers_retention,
            init_config.minimum_rotation_delay,
            current_epochs,
            current_timestamp,
        );
        let (_, bump) = get_gateway_root_config_internal(program_id);
        let config = GatewayConfig::new(
            bump,
            auth_weighted,
            init_config.operator,
            init_config.domain_separator,
        );

        // Check: Gateway Config account uses the canonical bump.
        assert_valid_gateway_root_pda(config.bump, gateway_root_pda.key)?;

        // Check: Gateway Config account is not initialized.
        gateway_root_pda.check_uninitialized_pda()?;

        let bump = config.bump;
        program_utils::init_pda(
            payer,
            gateway_root_pda,
            program_id,
            system_account,
            config,
            &[seed_prefixes::GATEWAY_SEED, &[bump]],
        )
    }
}
