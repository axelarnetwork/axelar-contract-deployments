use gas_service::get_gas_service_root_pda;
use gateway::get_gateway_root_config_pda;
use program_utils::{check_program_account, init_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::state::RootPDA;
use crate::{check_id, get_interchain_token_service_root_pda_internal};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let interchain_token_service_root_pda = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;
        let gas_service_root_pda = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        // TODO add interchain_address_tracker_pda
        assert_gateway_root_pda(gateway_root_pda);
        assert_gas_service_root_pda(gas_service_root_pda);

        interchain_token_service_root_pda.check_uninitialized_pda()?;
        let (derived, bump_seed) = get_interchain_token_service_root_pda_internal(
            gateway_root_pda.key,
            gas_service_root_pda.key,
            program_id,
        );
        if derived != *interchain_token_service_root_pda.key {
            return Err(ProgramError::InvalidSeeds);
        }

        // TODO we need to instantiate a global operator group here, which will have
        // operator-only access to ITS

        init_pda(
            funder_info,
            interchain_token_service_root_pda,
            program_id,
            system_program_info,
            RootPDA::new(bump_seed),
            &[
                &gateway_root_pda.key.to_bytes(),
                &gas_service_root_pda.key.to_bytes(),
                &[bump_seed],
            ],
        )?;
        Ok(())
    }
}

pub(crate) fn assert_gas_service_root_pda(gas_service_root_pda: &AccountInfo<'_>) {
    let (derived_gas_service_root_pda, _) = get_gas_service_root_pda();
    assert_eq!(
        derived_gas_service_root_pda, *gas_service_root_pda.key,
        "Gas service root account is not derived from gas service root id"
    );
    assert_eq!(
        &gas_service::ID,
        gas_service_root_pda.owner,
        "Gas service root account is not owned by the gas program"
    );
}

fn assert_gateway_root_pda(gateway_root_pda: &AccountInfo<'_>) {
    let (derived_gateway_config, _) = get_gateway_root_config_pda();
    assert_eq!(
        derived_gateway_config, *gateway_root_pda.key,
        "Gateway root account is not derived from gateway root id"
    );
    assert_eq!(
        &gateway::ID,
        gateway_root_pda.owner,
        "Gateway root account is not owned by the gateway program"
    );
}
