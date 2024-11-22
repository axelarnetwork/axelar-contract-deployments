//! Processor for [`TokenManager`] related requests.

use axelar_rkyv_encoding::types::PublicKey;
use interchain_token_transfer_gmp::DeployTokenManager;
use program_utils::{StorableArchive, ValidPDA};
use role_management::processor::{ensure_roles, RoleManagementAccounts};
use role_management::state::{Roles, UserRoles};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_option::COption;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions};
use spl_token_2022::state::Mint;

use super::LocalAction;
use crate::instructions::{self, Bumps};
use crate::seed_prefixes;
use crate::state::token_manager::{self, TokenManager};

impl LocalAction for DeployTokenManager {
    fn process_local_action<'a>(
        self,
        payer: &AccountInfo<'a>,
        accounts: &[AccountInfo<'a>],
        bumps: Bumps,
    ) -> ProgramResult {
        process_deploy(payer, accounts, &self, bumps)
    }
}

#[allow(clippy::todo)]
pub(crate) fn process_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instructions::token_manager::Instruction,
) -> ProgramResult {
    match instruction {
        instructions::token_manager::Instruction::SetFlowLimit { flow_limit } => {
            let instruction_accounts = SetFlowLimitAccounts::try_from(accounts)?;
            if !instruction_accounts.flow_limiter.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            set_flow_limit(&instruction_accounts, flow_limit)
        }
        instructions::token_manager::Instruction::AddFlowLimiter(inputs) => {
            if !inputs.roles.eq(&Roles::FLOW_LIMITER) {
                return Err(ProgramError::InvalidInstructionData);
            }

            let instruction_accounts = RoleManagementAccounts::try_from(accounts)?;
            role_management::processor::add(
                &crate::id(),
                instruction_accounts,
                &inputs,
                Roles::OPERATOR,
            )
        }
        instructions::token_manager::Instruction::RemoveFlowLimiter(inputs) => {
            if !inputs.roles.eq(&Roles::FLOW_LIMITER) {
                return Err(ProgramError::InvalidInstructionData);
            }

            let instruction_accounts = RoleManagementAccounts::try_from(accounts)?;
            role_management::processor::remove(
                &crate::id(),
                instruction_accounts,
                &inputs,
                Roles::OPERATOR,
            )
        }
        instructions::token_manager::Instruction::OperatorInstruction(operator_instruction) => {
            process_operator_instruction(accounts, operator_instruction)
        }
    }
}

/// Processes a [`DeployTokenManager`] GMP message.
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub(crate) fn process_deploy<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    payload: &DeployTokenManager,
    bumps: Bumps,
) -> ProgramResult {
    let token_manager_type: token_manager::Type = payload.token_manager_type.try_into()?;
    if token_manager::Type::NativeInterchainToken == token_manager_type {
        return Err(ProgramError::InvalidInstructionData);
    }

    let Ok((operator, token_address)) = token_manager::decode_params(payload.params.as_ref())
    else {
        msg!("Failed to decode operator and token address");
        return Err(ProgramError::InvalidInstructionData);
    };

    let deploy_token_manager = DeployTokenManagerInternal::new(
        payload.token_manager_type.try_into()?,
        payload.token_id.0,
        token_address,
        operator,
        None,
    );

    deploy(payer, accounts, bumps, &deploy_token_manager)
}

pub(crate) fn set_flow_limit(
    accounts: &SetFlowLimitAccounts<'_>,
    flow_limit: u64,
) -> ProgramResult {
    ensure_roles(
        &crate::id(),
        accounts.token_manager_pda,
        accounts.flow_limiter,
        accounts.token_manager_user_roles_pda,
        Roles::FLOW_LIMITER,
    )?;

    let mut token_manager = TokenManager::load(&crate::id(), accounts.token_manager_pda)?;
    token_manager.flow_limit = flow_limit;
    token_manager.store(accounts.token_manager_pda)?;

    Ok(())
}

pub(crate) struct DeployTokenManagerInternal {
    manager_type: token_manager::Type,
    token_id: PublicKey,
    token_address: PublicKey,
    operator: Option<Pubkey>,
    #[allow(dead_code)] // TODO: Remove this once we implement the minters logic.
    minter: Option<Pubkey>,
}

impl DeployTokenManagerInternal {
    pub(crate) fn new(
        manager_type: token_manager::Type,
        token_id: [u8; 32],
        token_address: Pubkey,
        operator: Option<Pubkey>,
        minter: Option<Pubkey>,
    ) -> Self {
        Self {
            manager_type,
            token_id: PublicKey::new_ed25519(token_id),
            token_address: PublicKey::new_ed25519(token_address.to_bytes()),
            operator,
            minter,
        }
    }
}

/// Deploys a new [`TokenManager`] PDA.
///
/// # Errors
///
/// An error occurred when deploying the [`TokenManager`] PDA. The reason can be
/// derived from the logs.
pub(crate) fn deploy<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    bumps: Bumps,
    deploy_token_manager: &DeployTokenManagerInternal,
) -> ProgramResult {
    check_accounts(accounts)?;

    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let token_manager_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let its_roles_pda = next_account_info(accounts_iter)?;
    let _rent_sysvar = next_account_info(accounts_iter)?;
    let operator = next_account_info(accounts_iter).ok();
    let operator_roles_pda = next_account_info(accounts_iter).ok();

    validate_token_manager_type(
        deploy_token_manager.manager_type,
        token_mint,
        token_manager_pda,
    )?;

    crate::create_associated_token_account(
        payer,
        token_mint,
        token_manager_ata,
        token_manager_pda,
        system_account,
        token_program,
    )?;

    let (interchain_token_pda, _) = crate::create_interchain_token_pda(
        its_root_pda.key,
        deploy_token_manager.token_id.as_ref(),
        bumps.interchain_token_pda_bump,
    );
    let (_token_manager_pda, bump) =
        crate::create_token_manager_pda(&interchain_token_pda, bumps.token_manager_pda_bump);
    let token_manager_ata = PublicKey::new_ed25519(token_manager_ata.key.to_bytes());

    if let (Some(operator), Some(operator_roles_pda), Some(encoded_operator)) =
        (operator, operator_roles_pda, deploy_token_manager.operator)
    {
        if encoded_operator.ne(operator.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        setup_roles(
            payer,
            token_manager_pda,
            operator,
            operator_roles_pda,
            system_account,
            Roles::OPERATOR | Roles::FLOW_LIMITER,
        )?;
    }
    setup_roles(
        payer,
        token_manager_pda,
        its_root_pda,
        its_roles_pda,
        system_account,
        Roles::OPERATOR | Roles::FLOW_LIMITER,
    )?;

    // TODO: Use the role management crate for this.
    //
    // let minters = match deploy_token_manager.token_manager_type {
    //     token_manager::Type::NativeInterchainToken
    //     | token_manager::Type::MintBurn
    //     | token_manager::Type::MintBurnFrom => deploy_token_manager
    //         .additional_minter
    //         .map(|minter| vec![PublicKey::new_ed25519(minter.key.to_bytes())]),
    //     token_manager::Type::LockUnlock | token_manager::Type::LockUnlockFee =>
    // None, };

    let token_manager = TokenManager::new(
        deploy_token_manager.manager_type,
        deploy_token_manager.token_id,
        deploy_token_manager.token_address,
        token_manager_ata,
        bump,
    );

    program_utils::init_rkyv_pda::<{ TokenManager::LEN }, _>(
        payer,
        token_manager_pda,
        &crate::id(),
        system_account,
        token_manager,
        &[
            seed_prefixes::TOKEN_MANAGER_SEED,
            interchain_token_pda.as_ref(),
            &[bump],
        ],
    )?;

    Ok(())
}

fn setup_roles<'a>(
    payer: &AccountInfo<'a>,
    token_manager_pda: &AccountInfo<'a>,
    user: &AccountInfo<'a>,
    user_roles_pda: &AccountInfo<'a>,
    system_account: &AccountInfo<'a>,
    roles: Roles,
) -> ProgramResult {
    let (derived_operator_roles_pda, operator_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), token_manager_pda.key, user.key);

    if derived_operator_roles_pda.ne(user_roles_pda.key) {
        return Err(ProgramError::InvalidAccountData);
    }

    let operator_roles = UserRoles::new(roles, operator_roles_pda_bump);
    operator_roles.init(
        &crate::id(),
        system_account,
        payer,
        user_roles_pda,
        &[
            role_management::seed_prefixes::USER_ROLES_SEED,
            token_manager_pda.key.as_ref(),
            user.key.as_ref(),
            &[operator_roles_pda_bump],
        ],
    )?;

    Ok(())
}

fn process_operator_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instructions::operator::Instruction,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let interchain_token_pda = next_account_info(accounts_iter)?;
    let role_management_accounts = RoleManagementAccounts::try_from(accounts_iter.as_slice())?;
    let token_manager = TokenManager::load(&crate::id(), role_management_accounts.resource)?;
    let (derived_token_manager_pda, _) =
        crate::create_token_manager_pda(interchain_token_pda.key, token_manager.bump);

    if derived_token_manager_pda.ne(role_management_accounts.resource.key) {
        msg!("Invalid token manager PDA provided");
        return Err(ProgramError::InvalidAccountData);
    }

    match instruction {
        instructions::operator::Instruction::TransferOperatorship(inputs) => {
            role_management::processor::transfer(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::OPERATOR,
            )?;
        }
        instructions::operator::Instruction::ProposeOperatorship(inputs) => {
            role_management::processor::propose(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::OPERATOR,
            )?;
        }
        instructions::operator::Instruction::AcceptOperatorship(inputs) => {
            role_management::processor::accept(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::empty(),
            )?;
        }
    }

    Ok(())
}

fn check_accounts(accounts: &[AccountInfo<'_>]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let ata_program = next_account_info(accounts_iter)?;

    if !system_program::check_id(system_account.key) {
        msg!("Invalid system account provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    if its_root_pda
        .check_initialized_pda_without_deserialization(&crate::id())
        .is_err()
    {
        msg!("ITS root PDA is not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    if token_manager_pda.check_uninitialized_pda().is_err() {
        msg!("TokenManager PDA is already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if spl_token_2022::check_spl_token_program_account(token_mint.owner).is_err() {
        msg!("Invalid token mint account provided");
        return Err(ProgramError::InvalidAccountData);
    }

    if token_program.key != token_mint.owner {
        msg!("Mint and program account mismatch");
        return Err(ProgramError::IncorrectProgramId);
    }

    if !spl_associated_token_account::check_id(ata_program.key) {
        msg!("Invalid associated token account program provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub(crate) fn validate_token_manager_type(
    ty: token_manager::Type,
    token_mint: &AccountInfo<'_>,
    token_manager_pda: &AccountInfo<'_>,
) -> ProgramResult {
    let mint_data = token_mint.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    // TODO: There's more logic required here, possibly some check on
    // the TokenManager being the delegate of some account, etc. It's still not
    // clear to me and I think it will become clearer when we start working on the
    // deployment of the token itself and the the transfers.
    match (mint.base.mint_authority, ty) {
        (
            COption::None,
            token_manager::Type::MintBurn
            | token_manager::Type::MintBurnFrom
            | token_manager::Type::NativeInterchainToken,
        ) => {
            msg!("Mint authority is required for MintBurn and MintBurnFrom tokens");
            Err(ProgramError::InvalidInstructionData)
        }
        (
            COption::Some(key),
            token_manager::Type::MintBurn
            | token_manager::Type::MintBurnFrom
            | token_manager::Type::NativeInterchainToken,
        ) if &key != token_manager_pda.key => {
            msg!("TokenManager is not the mint authority, which is required for this token type");
            Err(ProgramError::InvalidInstructionData)
        }
        (_, token_manager::Type::LockUnlockFee)
            if !mint
                .get_extension_types()?
                .contains(&ExtensionType::TransferFeeConfig) =>
        {
            msg!("The mint is not compatible with the LockUnlockFee TokenManager type, please make sure the mint has the TransferFeeConfig extension initialized");
            Err(ProgramError::InvalidAccountData)
        }
        _ => Ok(()),
    }
}

// TODO: Remove this once we uncomment the ensure_roles call in set_flow_limit.
#[allow(dead_code)]
pub(crate) struct SetFlowLimitAccounts<'a> {
    pub(crate) flow_limiter: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) its_user_roles_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_user_roles_pda: &'a AccountInfo<'a>,
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for SetFlowLimitAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error> {
        let accounts_iter = &mut value.iter();

        Ok(Self {
            flow_limiter: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            its_user_roles_pda: next_account_info(accounts_iter)?,
            token_manager_user_roles_pda: next_account_info(accounts_iter)?,
        })
    }
}
