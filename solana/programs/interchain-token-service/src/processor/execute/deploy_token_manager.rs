use interchain_token_transfer_gmp::DeployTokenManager;
use program_utils::check_program_account;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::pubkey::Pubkey;
use token_manager::instruction::Setup;
use token_manager::TokenManagerType;

use super::Processor;
use crate::state::RootPDA;
use crate::{check_id, get_flow_limiters_permission_group_id, get_operators_permission_group_id};

impl Processor {
    /// Processes an instruction.
    pub(super) fn relayer_gmp_deploy_token_manager(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: DeployTokenManager,
        _root_pda: &RootPDA,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();
        // Accounts provided for ValidateContractCall
        let _gateway_approved_message_pda = next_account_info(account_info_iter)?;
        let _signing_pda = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;
        let _gateway_program_id = next_account_info(account_info_iter)?;

        // Default `executable` accounts
        let its_root_pda = next_account_info(account_info_iter)?;
        let _gas_service_root_pda = next_account_info(account_info_iter)?;

        // Accounts specific for this ix
        let funder = next_account_info(account_info_iter)?;
        let token_manager_root_pda = next_account_info(account_info_iter)?;
        let operators_permission_group_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda_owner = next_account_info(account_info_iter)?;
        let flow_limiters_permission_group_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda_owner = next_account_info(account_info_iter)?;
        let token_mint = next_account_info(account_info_iter)?;
        let token_manager_ata = next_account_info(account_info_iter)?;

        // Executable accounts
        let system_program = next_account_info(account_info_iter)?;
        let _account_group_program = next_account_info(account_info_iter)?;
        let _token_manager_program = next_account_info(account_info_iter)?;
        let spl_associated_token_account_program = next_account_info(account_info_iter)?;
        let spl_token_program = next_account_info(account_info_iter)?;

        // TODO assert that `token_mint` is the same as `input.token_id`

        // Instantiate 2 new permission groups
        // Instantiate operator group
        invoke(
            &account_group::instruction::build_setup_permission_group_instruction(
                funder.key,
                operators_permission_group_pda.key,
                operators_permission_pda.key,
                operators_permission_pda_owner.key,
                get_operators_permission_group_id(&input.token_id, its_root_pda.key),
            )?,
            &[
                funder.clone(),
                operators_permission_group_pda.clone(),
                operators_permission_pda.clone(),
                operators_permission_pda_owner.clone(),
                system_program.clone(),
            ],
        )?;
        // Instantiate flow limiter group
        invoke(
            &account_group::instruction::build_setup_permission_group_instruction(
                funder.key,
                flow_limiters_permission_group_pda.key,
                flow_limiters_permission_pda.key,
                flow_limiters_permission_pda_owner.key,
                get_flow_limiters_permission_group_id(&input.token_id, its_root_pda.key),
            )?,
            &[
                funder.clone(),
                flow_limiters_permission_group_pda.clone(),
                flow_limiters_permission_pda.clone(),
                flow_limiters_permission_pda_owner.clone(),
                system_program.clone(),
            ],
        )?;

        // Instantiate a new TokenManager
        let token_manager_type =
            TokenManagerType::try_from(input.token_manager_type.as_usize() as u8)?;
        invoke(
            &token_manager::instruction::build_setup_instruction(
                funder.key,
                token_manager_root_pda.key,
                operators_permission_group_pda.key,
                operators_permission_pda_owner.key,
                flow_limiters_permission_group_pda.key,
                flow_limiters_permission_pda_owner.key,
                its_root_pda.key,
                token_mint.key,
                gateway_root_pda.key,
                Setup {
                    flow_limit: 0,
                    token_manager_type,
                },
            )?,
            &[
                funder.clone(),
                gateway_root_pda.clone(),
                token_manager_root_pda.clone(),
                operators_permission_group_pda.clone(),
                operators_permission_pda.clone(),
                operators_permission_pda_owner.clone(),
                flow_limiters_permission_group_pda.clone(),
                flow_limiters_permission_pda.clone(),
                flow_limiters_permission_pda_owner.clone(),
                its_root_pda.clone(),
                token_mint.clone(),
                token_manager_ata.clone(),
                system_program.clone(),
                spl_associated_token_account_program.clone(),
                spl_token_program.clone(),
            ],
        )?;

        Ok(())
    }
}
