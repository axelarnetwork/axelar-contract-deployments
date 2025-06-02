//! Module that handles the processing of the `InterchainToken` deployment.

use alloy_primitives::Bytes;
use axelar_solana_gateway::num_traits::Zero;
use event_utils::Event as _;
use interchain_token_transfer_gmp::{DeployInterchainToken, GMPPayload};
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1CpiBuilder;
use mpl_token_metadata::types::TokenStandard;
use program_utils::{
    pda::BorshPda, validate_mpl_token_metadata_key, validate_rent_key,
    validate_spl_associated_token_account_key, validate_system_account_key,
    validate_sysvar_instructions_key,
};
use role_management::processor::{
    ensure_roles, ensure_signer_roles, RoleAddAccounts, RoleRemoveAccounts,
    RoleTransferWithProposalAccounts,
};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed, set_return_data};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack as _;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};
use spl_token_2022::check_spl_token_program_account;
use spl_token_2022::instruction::initialize_mint;
use spl_token_2022::state::Mint;

use super::gmp::{self, GmpAccounts};
use super::token_manager::{DeployTokenManagerAccounts, DeployTokenManagerInternal};
use crate::state::deploy_approval::DeployApproval;
use crate::state::token_manager::{self, TokenManager};
use crate::state::InterchainTokenService;
use crate::{assert_valid_deploy_approval_pda, event, Validate};
use crate::{
    assert_valid_its_root_pda, assert_valid_token_manager_pda, seed_prefixes, FromAccountInfoSlice,
    Roles,
};

#[derive(Debug)]
pub(crate) struct DeployInterchainTokenAccounts<'a> {
    pub(crate) system_account: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) token_mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) its_roles_pda: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) sysvar_instructions: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata_program: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata_account: &'a AccountInfo<'a>,
    pub(crate) payer_ata: &'a AccountInfo<'a>,
    pub(crate) minter: Option<&'a AccountInfo<'a>>,
    pub(crate) minter_roles_pda: Option<&'a AccountInfo<'a>>,
}

impl Validate for DeployInterchainTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_account.key)?;
        check_spl_token_program_account(self.token_program.key)?;
        validate_spl_associated_token_account_key(self.ata_program.key)?;
        validate_rent_key(self.rent_sysvar.key)?;
        validate_sysvar_instructions_key(self.sysvar_instructions.key)?;
        validate_mpl_token_metadata_key(self.mpl_token_metadata_program.key)?;
        Ok(())
    }
}

impl<'a> FromAccountInfoSlice<'a> for DeployInterchainTokenAccounts<'a> {
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
            sysvar_instructions: next_account_info(accounts_iter)?,
            mpl_token_metadata_program: next_account_info(accounts_iter)?,
            mpl_token_metadata_account: next_account_info(accounts_iter)?,
            payer_ata: next_account_info(accounts_iter)?,
            minter: next_account_info(accounts_iter).ok(),
            minter_roles_pda: next_account_info(accounts_iter).ok(),
        })
    }
}

impl<'a> From<DeployInterchainTokenAccounts<'a>> for DeployTokenManagerAccounts<'a> {
    fn from(value: DeployInterchainTokenAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            its_root_pda: value.its_root_pda,
            token_manager_pda: value.token_manager_pda,
            token_mint: value.token_mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            its_roles_pda: value.its_roles_pda,
            rent_sysvar: value.rent_sysvar,
            operator: value.minter,
            operator_roles_pda: value.minter_roles_pda,
        }
    }
}

pub(crate) fn process_deploy<'a>(
    accounts: &'a [AccountInfo<'a>],
    salt: [u8; 32],
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> ProgramResult {
    let (payer, other_accounts) = accounts
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let deploy_salt = crate::interchain_token_deployer_salt(payer.key, &salt);
    let token_id = crate::interchain_token_id_internal(&deploy_salt);
    let parsed_accounts =
        DeployInterchainTokenAccounts::from_account_info_slice(other_accounts, &())?;
    if initial_supply.is_zero() && parsed_accounts.minter.is_none() {
        return Err(ProgramError::InvalidArgument);
    }

    event::InterchainTokenIdClaimed {
        token_id,
        deployer: *payer.key,
        salt: deploy_salt,
    }
    .emit();

    process_inbound_deploy(
        payer,
        parsed_accounts,
        token_id,
        name,
        symbol,
        decimals,
        initial_supply,
    )?;

    set_return_data(&token_id);

    Ok(())
}

pub(crate) fn process_inbound_deploy<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: DeployInterchainTokenAccounts<'a>,
    token_id: [u8; 32],
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> ProgramResult {
    msg!("Instruction: InboundDeploy");
    let its_root_pda_bump = InterchainTokenService::load(accounts.its_root_pda)?.bump;
    assert_valid_its_root_pda(accounts.its_root_pda, its_root_pda_bump)?;

    let (interchain_token_pda, interchain_token_pda_bump) =
        crate::find_interchain_token_pda(accounts.its_root_pda.key, &token_id);
    if interchain_token_pda.ne(accounts.token_mint.key) {
        msg!("Invalid mint account provided");
        return Err(ProgramError::InvalidArgument);
    }

    let (token_manager_pda, token_manager_pda_bump) =
        crate::find_token_manager_pda(accounts.its_root_pda.key, &token_id);
    if token_manager_pda.ne(accounts.token_manager_pda.key) {
        msg!("Invalid TokenManager account provided");
        return Err(ProgramError::InvalidArgument);
    }

    setup_mint(
        payer,
        &accounts,
        decimals,
        &token_id,
        interchain_token_pda_bump,
        token_manager_pda_bump,
        initial_supply,
    )?;
    setup_metadata(
        payer,
        &accounts,
        &token_id,
        name.clone(),
        symbol.clone(),
        String::new(),
        token_manager_pda_bump,
    )?;

    // The minter passed in the DeployInterchainToken call is used as the
    // `TokenManager` operator as well, see:
    // https://github.com/axelarnetwork/interchain-token-service/blob/v2.0.1/contracts/InterchainTokenService.sol#L758
    let deploy_token_manager = DeployTokenManagerInternal::new(
        token_manager::Type::NativeInterchainToken,
        token_id,
        *accounts.token_mint.key,
        accounts.minter.map(|account| *account.key),
        accounts.minter.map(|account| *account.key),
    );

    let deploy_token_manager_accounts = DeployTokenManagerAccounts::from(accounts);
    super::token_manager::deploy(
        payer,
        &deploy_token_manager_accounts,
        &deploy_token_manager,
        token_manager_pda_bump,
    )?;

    event::InterchainTokenDeployed {
        token_id,
        token_address: *deploy_token_manager_accounts.token_mint.key,
        minter: deploy_token_manager_accounts
            .operator
            .map(|account| *account.key)
            .unwrap_or_default(),
        name,
        symbol,
        decimals,
    }
    .emit();

    Ok(())
}

pub(crate) fn process_outbound_deploy<'a>(
    accounts: &'a [AccountInfo<'a>],
    salt: [u8; 32],
    destination_chain: String,
    maybe_destination_minter: Option<Vec<u8>>,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    const OUTBOUND_MESSAGE_ACCOUNTS_INDEX: usize = 3;
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let metadata = next_account_info(accounts_iter)?;
    let token_id = crate::interchain_token_id_internal(&salt);
    let mut outbound_message_accounts_index = OUTBOUND_MESSAGE_ACCOUNTS_INDEX;

    let destination_minter_data = if let Some(destination_minter) = maybe_destination_minter {
        let minter = next_account_info(accounts_iter)?;
        let deploy_approval = next_account_info(accounts_iter)?;
        let minter_roles_account = next_account_info(accounts_iter)?;
        let token_manager_account = next_account_info(accounts_iter)?;
        outbound_message_accounts_index = outbound_message_accounts_index.saturating_add(4);

        msg!("Instruction: OutboundDeployMinter");
        ensure_roles(
            &crate::id(),
            token_manager_account,
            minter,
            minter_roles_account,
            Roles::MINTER,
        )?;

        Some((Bytes::from(destination_minter), deploy_approval, minter))
    } else {
        None
    };

    let (_other, outbound_message_accounts) = accounts.split_at(outbound_message_accounts_index);
    let gmp_accounts = GmpAccounts::from_account_info_slice(outbound_message_accounts, &())?;
    msg!("Instruction: OutboundDeploy");

    let token_metadata = Metadata::from_bytes(&metadata.try_borrow_data()?)?;
    let mint_data = Mint::unpack(&mint.try_borrow_data()?)?;
    let name = token_metadata.name.trim_end_matches('\0').to_owned();
    let symbol = token_metadata.symbol.trim_end_matches('\0').to_owned();

    let deployment_started_event = event::InterchainTokenDeploymentStarted {
        token_id,
        token_name: name,
        token_symbol: symbol,
        token_decimals: mint_data.decimals,
        minter: destination_minter_data
            .as_ref()
            .map(|data| data.0.to_vec())
            .unwrap_or_default(),
        destination_chain: destination_chain.clone(),
    };
    deployment_started_event.emit();

    let message = GMPPayload::DeployInterchainToken(DeployInterchainToken {
        selector: DeployInterchainToken::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_id: token_id.into(),
        name: deployment_started_event.token_name,
        symbol: deployment_started_event.token_symbol,
        decimals: mint_data.decimals,
        minter: destination_minter_data
            .as_ref()
            .map(|data| data.0.clone())
            .unwrap_or_default(),
    });

    gmp::process_outbound(
        payer,
        &gmp_accounts,
        &message,
        destination_chain.clone(),
        gas_value,
        signing_pda_bump,
        None,
        true,
    )?;

    // This closes the account and transfers lamports back, thus, this needs to happen after all
    // CPIs
    if let Some((destination_minter, deploy_approval, minter)) = destination_minter_data {
        use_deploy_approval(
            minter,
            deploy_approval,
            &destination_minter,
            &token_id,
            &destination_chain,
        )?;
    }

    Ok(())
}

pub(crate) fn deploy_remote_interchain_token<'a>(
    accounts: &'a [AccountInfo<'a>],
    salt: [u8; 32],
    destination_chain: String,
    maybe_destination_minter: Option<Vec<u8>>,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let deploy_salt = crate::interchain_token_deployer_salt(payer.key, &salt);

    process_outbound_deploy(
        accounts,
        deploy_salt,
        destination_chain,
        maybe_destination_minter,
        gas_value,
        signing_pda_bump,
    )
}

pub(crate) fn deploy_remote_canonical_interchain_token<'a>(
    accounts: &'a [AccountInfo<'a>],
    destination_chain: String,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let _payer = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let deploy_salt = crate::canonical_interchain_token_deploy_salt(mint.key);

    process_outbound_deploy(
        accounts,
        deploy_salt,
        destination_chain,
        None,
        gas_value,
        signing_pda_bump,
    )
}

pub(crate) fn process_mint<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let mint = next_account_info(accounts_iter)?;
    let destination_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let minter = next_account_info(accounts_iter)?;
    let minter_roles_pda = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    msg!("Instruction: MintInterchainToken");
    let token_manager = TokenManager::load(token_manager_pda)?;
    assert_valid_token_manager_pda(
        token_manager_pda,
        its_root_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    if token_manager.token_address.as_ref() != mint.key.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }
    if mint.owner != token_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }

    ensure_signer_roles(
        &crate::id(),
        token_manager_pda,
        minter,
        minter_roles_pda,
        Roles::MINTER,
    )?;

    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            token_program.key,
            mint.key,
            destination_account.key,
            token_manager_pda.key,
            &[],
            amount,
        )?,
        &[
            mint.clone(),
            destination_account.clone(),
            token_manager_pda.clone(),
            token_program.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_pda.key.as_ref(),
            token_manager.token_id.as_ref(),
            &[token_manager.bump],
        ]],
    )?;
    Ok(())
}

fn setup_mint<'a>(
    payer: &AccountInfo<'a>,
    accounts: &DeployInterchainTokenAccounts<'a>,
    decimals: u8,
    token_id: &[u8],
    interchain_token_pda_bump: u8,
    token_manager_pda_bump: u8,
    initial_supply: u64,
) -> ProgramResult {
    let rent = Rent::get()?;

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            accounts.token_mint.key,
            rent.minimum_balance(spl_token_2022::state::Mint::LEN),
            spl_token_2022::state::Mint::LEN
                .try_into()
                .map_err(|_err| ProgramError::ArithmeticOverflow)?,
            accounts.token_program.key,
        ),
        &[
            payer.clone(),
            accounts.token_mint.clone(),
            accounts.system_account.clone(),
            accounts.token_program.clone(),
            accounts.token_manager_pda.clone(),
        ],
        &[&[
            seed_prefixes::INTERCHAIN_TOKEN_SEED,
            accounts.its_root_pda.key.as_ref(),
            token_id,
            &[interchain_token_pda_bump],
        ]],
    )?;

    invoke(
        &initialize_mint(
            &spl_token_2022::id(),
            accounts.token_mint.key,
            accounts.token_manager_pda.key,
            Some(accounts.token_manager_pda.key),
            decimals,
        )?,
        &[
            accounts.token_mint.clone(),
            accounts.rent_sysvar.clone(),
            accounts.token_manager_pda.clone(),
            accounts.token_program.clone(),
        ],
    )?;

    if initial_supply > 0 {
        crate::create_associated_token_account(
            payer,
            accounts.token_mint,
            accounts.payer_ata,
            payer,
            accounts.system_account,
            accounts.token_program,
        )?;

        invoke_signed(
            &spl_token_2022::instruction::mint_to(
                accounts.token_program.key,
                accounts.token_mint.key,
                accounts.payer_ata.key,
                accounts.token_manager_pda.key,
                &[],
                initial_supply,
            )?,
            &[
                payer.clone(),
                accounts.token_mint.clone(),
                accounts.payer_ata.clone(),
                accounts.token_manager_pda.clone(),
                accounts.token_program.clone(),
            ],
            &[&[
                seed_prefixes::TOKEN_MANAGER_SEED,
                accounts.its_root_pda.key.as_ref(),
                token_id,
                &[token_manager_pda_bump],
            ]],
        )?;
    }

    Ok(())
}

fn setup_metadata<'a>(
    payer: &AccountInfo<'a>,
    accounts: &DeployInterchainTokenAccounts<'a>,
    token_id: &[u8],
    name: String,
    symbol: String,
    uri: String,
    token_manager_pda_bump: u8,
) -> ProgramResult {
    CreateV1CpiBuilder::new(accounts.mpl_token_metadata_program)
        .metadata(accounts.mpl_token_metadata_account)
        .token_standard(TokenStandard::Fungible)
        .mint(accounts.token_mint, false)
        .authority(accounts.token_manager_pda)
        .update_authority(accounts.token_manager_pda, true)
        .payer(payer)
        .is_mutable(false)
        .name(name)
        .symbol(symbol)
        .uri(uri)
        .seller_fee_basis_points(0)
        .system_program(accounts.system_account)
        .sysvar_instructions(accounts.sysvar_instructions)
        .invoke_signed(&[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            accounts.its_root_pda.key.as_ref(),
            token_id,
            &[token_manager_pda_bump],
        ]])?;

    Ok(())
}

pub(crate) fn approve_deploy_remote_interchain_token(
    accounts: &[AccountInfo<'_>],
    deployer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
    destination_minter: Vec<u8>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let payer = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let deploy_approval_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;
    msg!("Instruction: ApproveDeployRemoteInterchainToken");

    ensure_signer_roles(
        &crate::id(),
        token_manager_account,
        payer,
        payer_roles_account,
        Roles::MINTER,
    )?;

    let token_id = crate::interchain_token_id(&deployer, &salt);
    let (_, bump) = crate::find_deployment_approval_pda(payer.key, &token_id, &destination_chain);

    let approval = DeployApproval {
        approved_destination_minter: solana_program::keccak::hash(&destination_minter).to_bytes(),
        bump,
    };

    approval.init(
        &crate::id(),
        system_account,
        payer,
        deploy_approval_account,
        &[
            seed_prefixes::DEPLOYMENT_APPROVAL_SEED,
            payer.key.as_ref(),
            &token_id,
            destination_chain.as_bytes(),
            &[bump],
        ],
    )?;

    event::DeployRemoteInterchainTokenApproval {
        minter: *payer.key,
        deployer,
        token_id,
        destination_chain,
        destination_minter,
    }
    .emit();

    Ok(())
}

pub(crate) fn revoke_deploy_remote_interchain_token(
    accounts: &[AccountInfo<'_>],
    deployer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let deploy_approval_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;
    msg!("Instruction: RevokeDeployRemoteInterchainToken");

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let token_id = crate::interchain_token_id(&deployer, &salt);
    let approval = DeployApproval::load(deploy_approval_account)?;

    assert_valid_deploy_approval_pda(
        deploy_approval_account,
        payer.key,
        &token_id,
        &destination_chain,
        approval.bump,
    )?;

    event::RevokeRemoteInterchainTokenApproval {
        minter: *payer.key,
        deployer,
        token_id,
        destination_chain,
    }
    .emit();

    program_utils::pda::close_pda(payer, deploy_approval_account)
}

pub(crate) fn use_deploy_approval(
    minter: &AccountInfo<'_>,
    deploy_approval_account: &AccountInfo<'_>,
    destination_minter: &[u8],
    token_id: &[u8; 32],
    destination_chain: &str,
) -> ProgramResult {
    let approval = DeployApproval::load(deploy_approval_account)?;

    assert_valid_deploy_approval_pda(
        deploy_approval_account,
        minter.key,
        token_id,
        destination_chain,
        approval.bump,
    )?;

    if approval.approved_destination_minter != solana_program::keccak::hash(destination_minter).0 {
        return Err(ProgramError::InvalidArgument);
    }

    program_utils::pda::close_pda(minter, deploy_approval_account)
}

pub(crate) fn process_transfer_mintership<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: TransferInterchainTokenMintership");

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
        Roles::MINTER,
        Roles::MINTER,
    )?;

    role_management::processor::remove(
        &crate::id(),
        role_remove_accounts,
        Roles::MINTER,
        Roles::MINTER,
    )
}

pub(crate) fn process_propose_mintership<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: ProposeInterchainTokenMintership");

    let accounts_iter = &mut accounts.iter();
    let its_config_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let its_config = InterchainTokenService::load(its_config_account)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_its_root_pda(its_config_account, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_config_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

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

    role_management::processor::propose(
        &crate::id(),
        role_management_accounts,
        Roles::MINTER,
        Roles::MINTER,
    )
}

pub(crate) fn process_accept_mintership<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: AcceptInterchainTokenMintership");

    let accounts_iter = &mut accounts.iter();
    let its_config_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let its_config = InterchainTokenService::load(its_config_account)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_its_root_pda(its_config_account, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_config_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

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

    role_management::processor::accept(
        &crate::id(),
        role_management_accounts,
        Roles::MINTER,
        Roles::empty(),
    )
}
