use interchain_token_transfer_gmp::DeployTokenManager;
use program_utils::{check_program_account, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::pubkey::Pubkey;
use token_manager::instruction::Setup;

use super::Processor;
use crate::state::RootPDA;
use crate::{check_id, get_flow_limiters_permission_group_id, get_operators_permission_group_id};

impl Processor {
    /// Processes an instruction.
    pub fn deploy_token_manager(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: DeployTokenManager,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        // Accounts to proxy
        let funder = next_account_info(account_info_iter)?;
        let token_manager_root_pda = next_account_info(account_info_iter)?;
        let operators_permission_group_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda_owner = next_account_info(account_info_iter)?;
        let flow_limiters_permission_group_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda_owner = next_account_info(account_info_iter)?;

        // Our accounts
        let its_root_pda = next_account_info(account_info_iter)?;

        // Executable accounts
        let system_program = next_account_info(account_info_iter)?;
        let _account_group_exec = next_account_info(account_info_iter)?;
        let _token_manager_exec = next_account_info(account_info_iter)?;

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

        // assert ITS root PDA
        its_root_pda.check_initialized_pda::<RootPDA>(program_id)?;

        // Instantiate a new TokenManager
        invoke(
            &token_manager::instruction::build_setup_instruction(
                funder.key,
                token_manager_root_pda.key,
                operators_permission_group_pda.key,
                operators_permission_pda.key,
                operators_permission_pda_owner.key,
                flow_limiters_permission_group_pda.key,
                flow_limiters_permission_pda.key,
                flow_limiters_permission_pda_owner.key,
                its_root_pda.key,
                Setup { flow_limit: 0 },
            )?,
            &[
                funder.clone(),
                token_manager_root_pda.clone(),
                operators_permission_group_pda.clone(),
                operators_permission_pda.clone(),
                operators_permission_pda_owner.clone(),
                flow_limiters_permission_group_pda.clone(),
                flow_limiters_permission_pda.clone(),
                flow_limiters_permission_pda_owner.clone(),
                its_root_pda.clone(),
                system_program.clone(),
            ],
        )?;

        // TODO port .postTokenManagerDeploy() code: https://github.com/axelarnetwork/interchain-token-service/blob/566e8504fe35ed63ae6c063dd8fd40a41fabc0c7/contracts/TokenHandler.sol#L159-L169

        // TODO create an ATA that's assigned to the token manager root PDA
        Ok(())
    }
}
