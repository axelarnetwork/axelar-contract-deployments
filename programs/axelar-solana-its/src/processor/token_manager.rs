//! Processor for [`TokenManager`] related requests.

use event_utils::Event as _;
use program_utils::{
    pda::{BorshPda, ValidPDA},
    validate_rent_key, validate_spl_associated_token_account_key, validate_system_account_key,
};
use role_management::processor::{
    ensure_roles, RoleAddAccounts, RoleRemoveAccounts, RoleTransferWithProposalAccounts,
};
use role_management::state::UserRoles;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::program_option::COption;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};
use spl_token_2022::check_spl_token_program_account;
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions};
use spl_token_2022::instruction::AuthorityType;
use spl_token_2022::state::Mint;

use crate::state::token_manager::{self, TokenManager};
use crate::state::InterchainTokenService;
use crate::{assert_valid_its_root_pda, event, Validate};
use crate::{assert_valid_token_manager_pda, seed_prefixes, FromAccountInfoSlice, Roles};

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

    let mut token_manager = TokenManager::load(accounts.token_manager_pda)?;
    token_manager.flow_slot.flow_limit = flow_limit;
    token_manager.store(
        accounts.flow_limiter,
        accounts.token_manager_pda,
        accounts.system_account,
    )?;

    Ok(())
}

pub(crate) struct DeployTokenManagerInternal {
    manager_type: token_manager::Type,
    token_id: [u8; 32],
    token_address: Pubkey,
    operator: Option<Pubkey>,
    minter: Option<Pubkey>,
}

impl DeployTokenManagerInternal {
    pub(crate) const fn new(
        manager_type: token_manager::Type,
        token_id: [u8; 32],
        token_address: Pubkey,
        operator: Option<Pubkey>,
        minter: Option<Pubkey>,
    ) -> Self {
        Self {
            manager_type,
            token_id,
            token_address,
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
    payer: &'a AccountInfo<'a>,
    accounts: &DeployTokenManagerAccounts<'a>,
    deploy_token_manager: &DeployTokenManagerInternal,
    token_manager_pda_bump: u8,
) -> ProgramResult {
    msg!("Instruction: TM Deploy");
    check_accounts(accounts)?;
    validate_mint_extensions(deploy_token_manager.manager_type, accounts.token_mint)?;

    crate::create_associated_token_account_idempotent(
        payer,
        accounts.token_mint,
        accounts.token_manager_ata,
        accounts.token_manager_pda,
        accounts.system_account,
        accounts.token_program,
    )?;

    if let Some(operator_from_message) = deploy_token_manager.operator {
        let (Some(operator), Some(operator_roles_pda)) =
            (accounts.operator, accounts.operator_roles_pda)
        else {
            return Err(ProgramError::InvalidArgument);
        };

        if operator_from_message.ne(operator.key) {
            msg!("Invalid operator provided");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut roles = Roles::OPERATOR | Roles::FLOW_LIMITER;
        if deploy_token_manager.minter.is_some()
            && deploy_token_manager.manager_type == token_manager::Type::NativeInterchainToken
        {
            roles |= Roles::MINTER;
        }

        setup_roles(
            payer,
            accounts.token_manager_pda,
            operator.key,
            operator_roles_pda,
            accounts.system_account,
            roles,
        )?;
    }

    setup_roles(
        payer,
        accounts.token_manager_pda,
        accounts.its_root_pda.key,
        accounts.its_roles_pda,
        accounts.system_account,
        Roles::OPERATOR | Roles::FLOW_LIMITER,
    )?;

    let token_manager = TokenManager::new(
        deploy_token_manager.manager_type,
        deploy_token_manager.token_id,
        deploy_token_manager.token_address,
        *accounts.token_manager_ata.key,
        token_manager_pda_bump,
    );
    token_manager.init(
        &crate::id(),
        accounts.system_account,
        payer,
        accounts.token_manager_pda,
        &[
            seed_prefixes::TOKEN_MANAGER_SEED,
            accounts.its_root_pda.key.as_ref(),
            &token_manager.token_id,
            &[token_manager.bump],
        ],
    )?;

    event::TokenManagerDeployed {
        token_id: deploy_token_manager.token_id,
        token_manager: *accounts.token_manager_pda.key,
        token_manager_type: deploy_token_manager.manager_type.into(),
        params: deploy_token_manager
            .operator
            .map(|op| op.to_bytes().to_vec())
            .unwrap_or_default(),
    }
    .emit();

    Ok(())
}

fn setup_roles<'a>(
    payer: &AccountInfo<'a>,
    token_manager_pda: &AccountInfo<'a>,
    user: &Pubkey,
    user_roles_pda: &AccountInfo<'a>,
    system_account: &AccountInfo<'a>,
    roles: Roles,
) -> ProgramResult {
    let (derived_user_roles_pda, user_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), token_manager_pda.key, user);

    if derived_user_roles_pda.ne(user_roles_pda.key) {
        msg!("Invalid user roles PDA provided");
        return Err(ProgramError::InvalidAccountData);
    }

    if let Ok(mut existing_roles) = UserRoles::<Roles>::load(user_roles_pda) {
        existing_roles.add(roles);
        existing_roles.store(payer, user_roles_pda, system_account)?;
    } else {
        let user_roles = UserRoles::new(roles, user_roles_pda_bump);
        user_roles.init(
            &crate::id(),
            system_account,
            payer,
            user_roles_pda,
            &[
                role_management::seed_prefixes::USER_ROLES_SEED,
                token_manager_pda.key.as_ref(),
                user.as_ref(),
                &[user_roles_pda_bump],
            ],
        )?;
    }

    Ok(())
}

fn check_accounts(accounts: &DeployTokenManagerAccounts<'_>) -> ProgramResult {
    if !system_program::check_id(accounts.system_account.key) {
        msg!("Invalid system account provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    if accounts
        .token_manager_pda
        .check_uninitialized_pda()
        .is_err()
    {
        msg!("TokenManager PDA is already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if spl_token_2022::check_spl_token_program_account(accounts.token_mint.owner).is_err() {
        msg!("Invalid token mint account provided");
        return Err(ProgramError::InvalidAccountData);
    }

    if accounts.token_program.key != accounts.token_mint.owner {
        msg!("Mint and program account mismatch");
        return Err(ProgramError::IncorrectProgramId);
    }

    if !spl_associated_token_account::check_id(accounts.ata_program.key) {
        msg!("Invalid associated token account program provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

pub(crate) fn validate_mint_extensions(
    ty: token_manager::Type,
    token_mint: &AccountInfo<'_>,
) -> ProgramResult {
    let mint_data = token_mint.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    if matches!(
        (
            ty,
            mint.get_extension_types()?
                .contains(&ExtensionType::TransferFeeConfig)
        ),
        (token_manager::Type::LockUnlock, true) | (token_manager::Type::LockUnlockFee, false)
    ) {
        msg!("The mint is not compatible with the type");
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(())
}

pub(crate) fn validate_token_manager_type(
    ty: token_manager::Type,
    token_mint: &AccountInfo<'_>,
    token_manager_pda: &AccountInfo<'_>,
) -> ProgramResult {
    let mint_data = token_mint.try_borrow_data()?;
    let mint = Mint::unpack_from_slice(&mint_data)?;

    match (mint.mint_authority, ty) {
        (
            COption::None,
            token_manager::Type::NativeInterchainToken
            | token_manager::Type::MintBurn
            | token_manager::Type::MintBurnFrom,
        ) => {
            msg!("Mint authority is required for the given token manager type");
            Err(ProgramError::InvalidInstructionData)
        }
        (
            COption::Some(key),
            token_manager::Type::NativeInterchainToken
            | token_manager::Type::MintBurn
            | token_manager::Type::MintBurnFrom,
        ) if &key != token_manager_pda.key => {
            msg!("TokenManager is not the mint authority, which is required for this token manager type");
            Err(ProgramError::InvalidInstructionData)
        }
        _ => Ok(()),
    }
}

pub(crate) fn handover_mint_authority(
    accounts: &[AccountInfo<'_>],
    token_id: [u8; 32],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let its_root = next_account_info(accounts_iter)?;
    let token_manager = next_account_info(accounts_iter)?;
    let minter_roles = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;
    msg!("Instruction: HandoverMintAuthority");
    let its_root_config = InterchainTokenService::load(its_root)?;
    let token_manager_config = TokenManager::load(token_manager)?;

    assert_valid_its_root_pda(its_root, its_root_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager,
        its_root.key,
        &token_id,
        token_manager_config.bump,
    )?;

    if token_manager_config.token_address != *mint.key {
        msg!("TokenManager PDA does not match the provided Mint account");
        return Err(ProgramError::InvalidAccountData);
    }

    let mint_authority = {
        let mint_data = mint.try_borrow_data()?;
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

        mint.base.mint_authority
    };

    match mint_authority {
        COption::None => {
            msg!("Cannot hand over mint authority of a TokenManager for non-mintable token");
            Err(ProgramError::InvalidArgument)
        }
        COption::Some(authority) if authority == *payer.key => {
            // The payer is the mint authority. The mint authority needs to be transferred
            // to the `TokenManager` and the `minter` role is added to the payer
            // on the `TokenManager`. Future minting by the user needs to go
            // through ITS.
            let authority_transfer_ix = spl_token_2022::instruction::set_authority(
                token_program.key,
                mint.key,
                Some(token_manager.key),
                AuthorityType::MintTokens,
                payer.key,
                &[],
            )?;

            invoke(&authority_transfer_ix, &[mint.clone(), payer.clone()])?;

            setup_roles(
                payer,
                token_manager,
                payer.key,
                minter_roles,
                system_account,
                Roles::MINTER,
            )?;

            Ok(())
        }
        COption::Some(_) => {
            msg!("Signer is not the mint authority");
            Err(ProgramError::InvalidArgument)
        }
    }?;

    Ok(())
}

#[derive(Debug)]
pub(crate) struct DeployTokenManagerAccounts<'a> {
    pub(crate) system_account: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) token_mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) its_roles_pda: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) operator: Option<&'a AccountInfo<'a>>,
    pub(crate) operator_roles_pda: Option<&'a AccountInfo<'a>>,
}

impl Validate for DeployTokenManagerAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_account.key)?;
        check_spl_token_program_account(self.token_program.key)?;
        validate_spl_associated_token_account_key(self.ata_program.key)?;
        validate_rent_key(self.rent_sysvar.key)?;
        Ok(())
    }
}

impl<'a> FromAccountInfoSlice<'a> for DeployTokenManagerAccounts<'a> {
    type Context = ();

    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        _context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut accounts.iter();

        Ok(Self {
            system_account: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            token_mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            its_roles_pda: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            operator: next_account_info(accounts_iter).ok(),
            operator_roles_pda: next_account_info(accounts_iter).ok(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct SetFlowLimitAccounts<'a> {
    pub(crate) flow_limiter: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) its_user_roles_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_user_roles_pda: &'a AccountInfo<'a>,
    pub(crate) system_account: &'a AccountInfo<'a>,
}

impl<'a> FromAccountInfoSlice<'a> for SetFlowLimitAccounts<'a> {
    type Context = ();

    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        _context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut accounts.iter();

        Ok(Self {
            flow_limiter: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            its_user_roles_pda: next_account_info(accounts_iter)?,
            token_manager_user_roles_pda: next_account_info(accounts_iter)?,
            system_account: next_account_info(accounts_iter)?,
        })
    }
}

impl Validate for SetFlowLimitAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_account.key)?;

        let its_config_account = InterchainTokenService::load(self.its_root_pda)?;
        assert_valid_its_root_pda(self.its_root_pda, its_config_account.bump)?;
        Ok(())
    }
}

pub(crate) fn process_add_flow_limiter<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: AddTokenManagerFlowLimiter");

    let accounts_iter = &mut accounts.iter();
    let its_config_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let resource = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;

    let its_config = InterchainTokenService::load(its_config_account)?;
    assert_valid_its_root_pda(its_config_account, its_config.bump)?;
    if resource.key == its_config_account.key {
        msg!("Resource is not a TokenManager");
        return Err(ProgramError::InvalidAccountData);
    }

    // Ensure resource is a `TokenManager`
    TokenManager::load(resource)?;

    let role_management_accounts = RoleAddAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource,
        destination_user_account,
        destination_roles_account,
    };

    role_management::processor::add(
        &crate::id(),
        role_management_accounts,
        Roles::FLOW_LIMITER,
        Roles::OPERATOR,
    )
}

pub(crate) fn process_remove_flow_limiter<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: RemoveTokenManagerFlowLimiter");

    let accounts_iter = &mut accounts.iter();
    let its_config_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let resource = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;

    let its_config = InterchainTokenService::load(its_config_account)?;
    assert_valid_its_root_pda(its_config_account, its_config.bump)?;
    if resource.key == its_config_account.key {
        msg!("Resource is not a TokenManager");
        return Err(ProgramError::InvalidAccountData);
    }

    // Ensure resource is a `TokenManager`
    TokenManager::load(resource)?;

    let role_management_accounts = RoleRemoveAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource,
        origin_user_account,
        origin_roles_account,
    };

    role_management::processor::remove(
        &crate::id(),
        role_management_accounts,
        Roles::FLOW_LIMITER,
        Roles::OPERATOR,
    )
}

pub(crate) fn process_set_flow_limit<'a>(
    accounts: &'a [AccountInfo<'a>],
    flow_limit: u64,
) -> ProgramResult {
    msg!("Instruction: SetTokenManagerFlowLimit");

    let instruction_accounts = SetFlowLimitAccounts::from_account_info_slice(accounts, &())?;
    if !instruction_accounts.flow_limiter.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    set_flow_limit(&instruction_accounts, flow_limit)
}

pub(crate) fn process_transfer_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: TransferTokenManagerOperatorship");

    let accounts_iter = &mut accounts.iter();
    let its_config_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;

    let its_config = InterchainTokenService::load(its_config_account)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_its_root_pda(its_config_account, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_config_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let role_add_accounts = RoleAddAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource: token_manager_account,
        destination_user_account,
        destination_roles_account,
    };
    let role_remove_accounts = RoleRemoveAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource: token_manager_account,
        origin_user_account: payer,
        origin_roles_account: payer_roles_account,
    };

    role_management::processor::add(
        &crate::id(),
        role_add_accounts,
        Roles::OPERATOR,
        Roles::OPERATOR,
    )?;

    role_management::processor::remove(
        &crate::id(),
        role_remove_accounts,
        Roles::OPERATOR,
        Roles::OPERATOR,
    )
}

pub(crate) fn process_propose_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: ProposeTokenManagerOperatorship");

    let accounts_iter = &mut accounts.iter();
    let its_config_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource: token_manager_account,
        destination_user_account,
        destination_roles_account,
        origin_user_account: payer,
        origin_roles_account: payer_roles_account,
        proposal_account,
    };

    let its_config = InterchainTokenService::load(its_config_account)?;
    assert_valid_its_root_pda(its_config_account, its_config.bump)?;
    let token_manager = TokenManager::load(role_management_accounts.resource)?;
    assert_valid_token_manager_pda(
        role_management_accounts.resource,
        its_config_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    role_management::processor::propose(&crate::id(), role_management_accounts, Roles::OPERATOR)
}

pub(crate) fn process_accept_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: AcceptTokenManagerOperatorship");

    let accounts_iter = &mut accounts.iter();
    let its_config_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource: token_manager_account,
        destination_user_account: payer,
        destination_roles_account: payer_roles_account,
        origin_user_account,
        origin_roles_account,
        proposal_account,
    };

    let its_config = InterchainTokenService::load(its_config_account)?;
    assert_valid_its_root_pda(its_config_account, its_config.bump)?;
    let token_manager = TokenManager::load(role_management_accounts.resource)?;
    assert_valid_token_manager_pda(
        role_management_accounts.resource,
        its_config_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    role_management::processor::accept(&crate::id(), role_management_accounts, Roles::OPERATOR)
}
