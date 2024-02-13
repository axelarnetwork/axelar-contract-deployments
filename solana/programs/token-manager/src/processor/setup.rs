//! Setup a new token manager

use account_group::state::{PermissionAccount, PermissionGroupAccount};
use program_utils::{check_program_account, init_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::pubkey::Pubkey;

use super::{
    assert_permission_group_pda, assert_permission_pda, assert_token_manager_account, Processor,
};
use crate::instruction::Setup;
use crate::state::TokenManagerRootAccount;
use crate::{check_id, TokenManagerType};

impl Processor {
    /// Sets up a new Token Manager with the provided parameters.
    pub fn process_setup(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        setup: Setup,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let token_manager_root_pda = next_account_info(account_info_iter)?;
        let operators_permission_group_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda_owner = next_account_info(account_info_iter)?;
        let flow_limiters_permission_group_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda_owner = next_account_info(account_info_iter)?;
        let service_program_pda = next_account_info(account_info_iter)?;
        let token_mint = next_account_info(account_info_iter)?;
        let token_manager_ata = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;

        let system_program = next_account_info(account_info_iter)?;
        let spl_associated_token_account_program = next_account_info(account_info_iter)?;
        let spl_token_program = next_account_info(account_info_iter)?;

        // Assert account groups
        let operator_group = operators_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(&account_group::ID)?
            .id;
        let flow_group = flow_limiters_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(&account_group::ID)?
            .id;
        let _perm_pda = operators_permission_pda
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)?;
        let _perm_pda = flow_limiters_permission_pda
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)?;
        assert_permission_group_pda(operator_group, operators_permission_group_pda);
        assert_permission_group_pda(flow_group, flow_limiters_permission_group_pda);
        assert_permission_pda(
            operators_permission_group_pda,
            operators_permission_pda,
            operators_permission_pda_owner,
        );
        assert_permission_pda(
            flow_limiters_permission_group_pda,
            flow_limiters_permission_pda,
            flow_limiters_permission_pda_owner,
        );

        // Assert token manager pdas
        token_manager_root_pda.check_uninitialized_pda()?;
        let token_manager_root_pda_bump = assert_token_manager_account(
            token_manager_root_pda,
            operators_permission_group_pda,
            flow_limiters_permission_group_pda,
            service_program_pda,
            program_id,
        )?;

        // Assert Gateway PDA
        gateway_root_pda.check_initialized_pda_without_deserialization(&gateway::id())?;

        // Initialize root PDA
        init_pda(
            funder_info,
            token_manager_root_pda,
            program_id,
            system_program,
            TokenManagerRootAccount {
                flow_limit: setup.flow_limit,
                token_manager_type: setup.token_manager_type.clone(),
                token_mint: *token_mint.key,
                associated_token_account: *token_manager_ata.key,
            },
            &[
                &operators_permission_group_pda.key.to_bytes(),
                &flow_limiters_permission_group_pda.key.to_bytes(),
                &service_program_pda.key.to_bytes(),
                &[token_manager_root_pda_bump],
            ],
        )?;

        // Initialize ATA owned by TokenManager
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                funder_info.key,
                token_manager_root_pda.key,
                token_mint.key,
                spl_token_program.key,
            ),
            &[
                funder_info.clone(),
                token_manager_root_pda.clone(),
                token_mint.clone(),
                token_manager_ata.clone(),
                spl_token_program.clone(),
                system_program.clone(),
                spl_associated_token_account_program.clone(),
            ],
        )?;
        match setup.token_manager_type {
            TokenManagerType::LockUnlock | TokenManagerType::LockUnlockFee => {
                // Set Delegate to `service_program_pda`
                invoke_signed(
                    &spl_token::instruction::approve(
                        spl_token_program.key,
                        token_manager_ata.key,
                        service_program_pda.key,
                        token_manager_root_pda.key,
                        &[],
                        u64::MAX,
                    )
                    .unwrap(),
                    &[
                        spl_token_program.clone(),
                        token_manager_ata.clone(),
                        service_program_pda.clone(),
                        token_manager_root_pda.clone(),
                    ],
                    &[&[
                        &operators_permission_group_pda.key.to_bytes(),
                        &flow_limiters_permission_group_pda.key.to_bytes(),
                        &service_program_pda.key.to_bytes(),
                        &[token_manager_root_pda_bump],
                    ]],
                )?;
            }
            TokenManagerType::Gateway => {
                // Set Delegate to `gateway_pda`
                invoke_signed(
                    &spl_token::instruction::approve(
                        spl_token_program.key,
                        token_manager_ata.key,
                        gateway_root_pda.key,
                        token_manager_root_pda.key,
                        &[],
                        u64::MAX,
                    )
                    .unwrap(),
                    &[
                        spl_token_program.clone(),
                        token_manager_ata.clone(),
                        gateway_root_pda.clone(),
                        token_manager_root_pda.clone(),
                    ],
                    &[&[
                        &operators_permission_group_pda.key.to_bytes(),
                        &flow_limiters_permission_group_pda.key.to_bytes(),
                        &service_program_pda.key.to_bytes(),
                        &[token_manager_root_pda_bump],
                    ]],
                )?;
            }
            _ => {
                // Do nothing
            }
        }

        Ok(())
    }
}
