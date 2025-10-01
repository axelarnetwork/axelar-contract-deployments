//! Module that handles the processing of the `InterchainToken` deployment.

use alloy_primitives::Bytes;
use axelar_solana_gateway::num_traits::Zero;
use event_utils::Event as _;
use interchain_token_transfer_gmp::{DeployInterchainToken, GMPPayload};
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1CpiBuilder;
use mpl_token_metadata::types::TokenStandard;
use program_utils::pda::init_pda_raw;
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
use solana_program::msg;
use solana_program::program::{invoke, invoke_signed, set_return_data};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack as _;
use solana_program::pubkey::Pubkey;
use spl_token_2022::extension::metadata_pointer::MetadataPointer;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::instruction::initialize_mint;
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::state::TokenMetadata;

use super::gmp::{self, GmpAccounts};
use super::token_manager::{DeployTokenManagerAccounts, DeployTokenManagerInternal};
use crate::state::deploy_approval::DeployApproval;
use crate::state::token_manager::{self, TokenManager};
use crate::state::InterchainTokenService;
use crate::{
    assert_its_not_paused, assert_valid_deploy_approval_pda, events, find_its_root_pda, Validate,
};
use crate::{
    assert_valid_its_root_pda, assert_valid_token_manager_pda, seed_prefixes, FromAccountInfoSlice,
    Roles,
};

#[derive(Debug)]
pub(crate) struct DeployInterchainTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) deployer: &'a AccountInfo<'a>,
    pub(crate) system_account: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) token_mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) sysvar_instructions: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata_program: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata_account: &'a AccountInfo<'a>,
    pub(crate) deployer_ata: &'a AccountInfo<'a>,
    pub(crate) minter: Option<&'a AccountInfo<'a>>,
    pub(crate) minter_roles_pda: Option<&'a AccountInfo<'a>>,
}

impl Validate for DeployInterchainTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_account.key)?;
        validate_spl_associated_token_account_key(self.ata_program.key)?;
        validate_rent_key(self.rent_sysvar.key)?;
        validate_sysvar_instructions_key(self.sysvar_instructions.key)?;
        validate_mpl_token_metadata_key(self.mpl_token_metadata_program.key)?;
        spl_token_2022::check_program_account(self.token_program.key)?;

        if !self.payer.is_signer {
            msg!("Payer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !self.deployer.is_signer {
            msg!("Deployer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // If it's a cross-chain message, payer_ata is not set (i.e., is set to program id)
        if *self.deployer_ata.key != crate::id() {
            crate::assert_valid_ata(
                self.deployer_ata.key,
                self.token_program.key,
                self.token_mint.key,
                self.deployer.key,
            )?;
        }

        crate::assert_valid_ata(
            self.token_manager_ata.key,
            self.token_program.key,
            self.token_mint.key,
            self.token_manager_pda.key,
        )?;

        Ok(())
    }
}

impl<'a> FromAccountInfoSlice<'a> for DeployInterchainTokenAccounts<'a> {
    type Context = Option<&'a AccountInfo<'a>>;
    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        maybe_payer: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut accounts.iter();
        let (payer, deployer) = if let Some(payer) = maybe_payer {
            (*payer, *payer)
        } else {
            (
                next_account_info(accounts_iter)?,
                next_account_info(accounts_iter)?,
            )
        };

        Ok(Self {
            payer,
            deployer,
            system_account: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            token_mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            sysvar_instructions: next_account_info(accounts_iter)?,
            mpl_token_metadata_program: next_account_info(accounts_iter)?,
            mpl_token_metadata_account: next_account_info(accounts_iter)?,
            deployer_ata: next_account_info(accounts_iter)?,
            minter: next_account_info(accounts_iter).ok(),
            minter_roles_pda: next_account_info(accounts_iter).ok(),
        })
    }
}

impl<'a> From<DeployInterchainTokenAccounts<'a>> for DeployTokenManagerAccounts<'a> {
    fn from(value: DeployInterchainTokenAccounts<'a>) -> Self {
        Self {
            payer: value.payer,
            system_account: value.system_account,
            its_root_pda: value.its_root_pda,
            token_manager_pda: value.token_manager_pda,
            token_mint: value.token_mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
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
    let parsed_accounts = DeployInterchainTokenAccounts::from_account_info_slice(accounts, &None)?;
    let deploy_salt = crate::interchain_token_deployer_salt(parsed_accounts.deployer.key, &salt);
    let token_id = crate::interchain_token_id_internal(&deploy_salt);

    if initial_supply.is_zero() && parsed_accounts.minter.is_none() {
        return Err(ProgramError::InvalidArgument);
    }

    if name.len() > mpl_token_metadata::MAX_NAME_LENGTH
        || symbol.len() > mpl_token_metadata::MAX_SYMBOL_LENGTH
    {
        msg!("Name and/or symbol length too long");
        return Err(ProgramError::InvalidArgument);
    }

    events::InterchainTokenIdClaimed {
        token_id,
        deployer: *parsed_accounts.deployer.key,
        salt: deploy_salt,
    }
    .emit();

    process_inbound_deploy(
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
    accounts: DeployInterchainTokenAccounts<'a>,
    token_id: [u8; 32],
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> ProgramResult {
    msg!("Instruction: InboundDeploy");
    let its_config = InterchainTokenService::load(accounts.its_root_pda)?;
    assert_valid_its_root_pda(accounts.its_root_pda, its_config.bump)?;
    assert_its_not_paused(&its_config)?;

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
        &accounts,
        decimals,
        &token_id,
        interchain_token_pda_bump,
        token_manager_pda_bump,
        initial_supply,
    )?;

    let mut truncated_name = name;
    let mut truncated_symbol = symbol;
    truncated_name.truncate(mpl_token_metadata::MAX_NAME_LENGTH);
    truncated_symbol.truncate(mpl_token_metadata::MAX_SYMBOL_LENGTH);

    setup_metadata(
        &accounts,
        &token_id,
        truncated_name.clone(),
        truncated_symbol.clone(),
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
        &deploy_token_manager_accounts,
        &deploy_token_manager,
        token_manager_pda_bump,
    )?;

    events::InterchainTokenDeployed {
        token_id,
        token_address: *deploy_token_manager_accounts.token_mint.key,
        minter: deploy_token_manager_accounts
            .operator
            .map(|account| *account.key)
            .unwrap_or_default(),
        name: truncated_name,
        symbol: truncated_symbol,
        decimals,
    }
    .emit();

    Ok(())
}

/// Retrieves token metadata with fallback logic:
/// 1. First, try to get metadata from Token 2022 extensions
///     - If the metadata pointer points to the mint itself, we try to deserialize it using
///     `TokenMetadata`
/// 2. If we can't retrieve the metadata from embedded TokenMetadata, we try to deserialize the
///    data from the given metadata account, if any, as Metaplex `Metadata`.
pub(crate) fn get_token_metadata(
    mint: &AccountInfo,
    maybe_metadata_account: Option<&AccountInfo>,
) -> Result<(String, String), ProgramError> {
    let mint_data = mint.try_borrow_data()?;

    if let Ok(mint_with_extensions) = StateWithExtensions::<Mint>::unpack(&mint_data) {
        if let Ok(metadata_pointer) = mint_with_extensions.get_extension::<MetadataPointer>() {
            if let Some(metadata_address) =
                Option::<Pubkey>::from(metadata_pointer.metadata_address)
            {
                if metadata_address == *mint.key {
                    if let Ok(token_metadata_ext) =
                        mint_with_extensions.get_variable_len_extension::<TokenMetadata>()
                    {
                        return Ok((token_metadata_ext.name, token_metadata_ext.symbol));
                    }
                }
            }
        }
    }

    let metadata_account = maybe_metadata_account.ok_or(ProgramError::NotEnoughAccountKeys)?;
    if *metadata_account.owner != mpl_token_metadata::ID {
        msg!("Invalid Metaplex metadata account");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let token_metadata = Metadata::from_bytes(&metadata_account.try_borrow_data()?)?;
    if token_metadata.mint != *mint.key {
        msg!("The metadata and mint accounts passed don't match");
        return Err(ProgramError::InvalidArgument);
    }

    let name = token_metadata.name.trim_end_matches('\0').to_owned();
    let symbol = token_metadata.symbol.trim_end_matches('\0').to_owned();

    Ok((name, symbol))
}

pub(crate) fn process_outbound_deploy<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    salt: [u8; 32],
    destination_chain: String,
    maybe_destination_minter: Option<Vec<u8>>,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    const OUTBOUND_MESSAGE_ACCOUNTS_INDEX: usize = 3;
    let accounts_iter = &mut accounts.iter();
    let mint = next_account_info(accounts_iter)?;
    let metadata = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let token_id = crate::interchain_token_id_internal(&salt);
    let mut outbound_message_accounts_index = OUTBOUND_MESSAGE_ACCOUNTS_INDEX;

    let destination_minter_data = if let Some(destination_minter) = maybe_destination_minter {
        let minter = next_account_info(accounts_iter)?;
        let deploy_approval = next_account_info(accounts_iter)?;
        let minter_roles_account = next_account_info(accounts_iter)?;
        outbound_message_accounts_index = outbound_message_accounts_index.saturating_add(3);

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
    let its_root_config = InterchainTokenService::load(gmp_accounts.its_root_account)?;
    assert_valid_its_root_pda(gmp_accounts.its_root_account, its_root_config.bump)?;
    if destination_chain == its_root_config.chain_name {
        msg!("Cannot deploy remotely to the origin chain");
        return Err(ProgramError::InvalidInstructionData);
    }

    msg!("Instruction: OutboundDeploy");

    // Get metadata with fallback logic (Token 2022 extensions first, then Metaplex)
    let (name, symbol) = get_token_metadata(mint, Some(metadata))?;
    let mint_data_ref = mint.try_borrow_data()?;
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data_ref)?;
    let mint_data = mint_state.base;

    let token_manager = TokenManager::load(token_manager_account)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        gmp_accounts.its_root_account.key,
        &token_id,
        token_manager.bump,
    )?;
    if token_manager.token_address != *mint.key {
        msg!("TokenManager doesn't match mint");
        return Err(ProgramError::InvalidArgument);
    }

    let deployment_started_events = events::InterchainTokenDeploymentStarted {
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
    deployment_started_events.emit();

    let message = GMPPayload::DeployInterchainToken(DeployInterchainToken {
        selector: DeployInterchainToken::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_id: token_id.into(),
        name: deployment_started_events.token_name,
        symbol: deployment_started_events.token_symbol,
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
        true,
    )?;

    // This closes the account and transfers lamports back, thus, this needs to happen after all
    // CPIs
    if let Some((destination_minter, deploy_approval, minter)) = destination_minter_data {
        use_deploy_approval(
            payer,
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
    let ([payer, deployer], outbound_deploy_accounts) = accounts.split_at(2) else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer.is_signer {
        msg!("Payer should be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !deployer.is_signer {
        msg!("Deployer should be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let deploy_salt = crate::interchain_token_deployer_salt(deployer.key, &salt);

    process_outbound_deploy(
        payer,
        outbound_deploy_accounts,
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
    let ([payer], outbound_deploy_accounts) = accounts.split_at(1) else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer.is_signer {
        msg!("Payer should be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mint = outbound_deploy_accounts
        .first()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let deploy_salt = crate::canonical_interchain_token_deploy_salt(mint.key);

    process_outbound_deploy(
        payer,
        outbound_deploy_accounts,
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

    let its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(its_root_pda, its_root_config.bump)?;

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

    spl_token_2022::check_spl_token_program_account(token_program.key)?;

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
    accounts: &DeployInterchainTokenAccounts<'a>,
    decimals: u8,
    token_id: &[u8],
    interchain_token_pda_bump: u8,
    token_manager_pda_bump: u8,
    initial_supply: u64,
) -> ProgramResult {
    init_pda_raw(
        accounts.payer,
        accounts.token_mint,
        accounts.token_program.key,
        accounts.system_account,
        spl_token_2022::state::Mint::LEN
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        &[
            seed_prefixes::INTERCHAIN_TOKEN_SEED,
            accounts.its_root_pda.key.as_ref(),
            token_id,
            &[interchain_token_pda_bump],
        ],
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
        crate::create_associated_token_account_idempotent(
            accounts.payer,
            accounts.token_mint,
            accounts.deployer_ata,
            accounts.deployer,
            accounts.system_account,
            accounts.token_program,
        )?;

        invoke_signed(
            &spl_token_2022::instruction::mint_to(
                accounts.token_program.key,
                accounts.token_mint.key,
                accounts.deployer_ata.key,
                accounts.token_manager_pda.key,
                &[],
                initial_supply,
            )?,
            &[
                accounts.payer.clone(),
                accounts.deployer.clone(),
                accounts.token_mint.clone(),
                accounts.deployer_ata.clone(),
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
        .payer(accounts.payer)
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
    let minter = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let minter_roles_account = next_account_info(accounts_iter)?;
    let deploy_approval_account = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;
    msg!("Instruction: ApproveDeployRemoteInterchainToken");

    if !payer.is_signer {
        msg!("Payer should be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // This also ensures minter.is_signer == true
    ensure_signer_roles(
        &crate::id(),
        token_manager_account,
        minter,
        minter_roles_account,
        Roles::MINTER,
    )?;

    let token_id = crate::interchain_token_id(&deployer, &salt);
    let (its_root_pda, _) = find_its_root_pda();
    let token_manager = TokenManager::load(token_manager_account)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        &its_root_pda,
        &token_id,
        token_manager.bump,
    )?;

    let (deploy_approval_pda, bump) =
        crate::find_deployment_approval_pda(minter.key, &token_id, &destination_chain);
    if deploy_approval_pda != *deploy_approval_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let approval = DeployApproval {
        approved_destination_minter: solana_program::keccak::hash(&destination_minter).to_bytes(),
        bump,
    };

    let destination_chain_hash =
        solana_program::keccak::hashv(&[destination_chain.as_bytes()]).to_bytes();
    approval.init(
        &crate::id(),
        system_account,
        payer,
        deploy_approval_account,
        &[
            seed_prefixes::DEPLOYMENT_APPROVAL_SEED,
            minter.key.as_ref(),
            &token_id,
            destination_chain_hash.as_ref(),
            &[bump],
        ],
    )?;

    events::DeployRemoteInterchainTokenApproval {
        minter: *minter.key,
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
    let minter = next_account_info(accounts_iter)?;
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
        minter.key,
        &token_id,
        &destination_chain,
        approval.bump,
    )?;

    events::RevokeRemoteInterchainTokenApproval {
        minter: *minter.key,
        deployer,
        token_id,
        destination_chain,
    }
    .emit();

    program_utils::pda::close_pda(payer, deploy_approval_account, &crate::id())
}

pub(crate) fn use_deploy_approval<'a>(
    payer: &AccountInfo<'a>,
    minter: &AccountInfo<'a>,
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

    program_utils::pda::close_pda(payer, deploy_approval_account, &crate::id())
}

pub(crate) fn process_transfer_mintership<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: TransferInterchainTokenMintership");

    let accounts_iter = &mut accounts.iter();
    let its_config_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let sender_user_account = next_account_info(accounts_iter)?;
    let sender_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;

    if sender_user_account.key == destination_user_account.key {
        msg!("Source and destination accounts are the same");
        return Err(ProgramError::InvalidArgument);
    }

    let its_config = InterchainTokenService::load(its_config_pda)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_its_root_pda(its_config_pda, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_config_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let role_add_accounts = RoleAddAccounts {
        system_account,
        payer,
        authority_user_account: sender_user_account,
        authority_roles_account: sender_roles_account,
        resource: token_manager_account,
        target_user_account: destination_user_account,
        target_roles_account: destination_roles_account,
    };

    let role_remove_accounts = RoleRemoveAccounts {
        system_account,
        payer,
        authority_user_account: sender_user_account,
        authority_roles_account: sender_roles_account,
        resource: token_manager_account,
        target_user_account: sender_user_account,
        target_roles_account: sender_roles_account,
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
    let its_config_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;

    let its_config = InterchainTokenService::load(its_config_pda)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_its_root_pda(its_config_pda, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_config_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account,
        payer,
        origin_user_account,
        origin_roles_account,
        resource: token_manager_account,
        destination_user_account,
        destination_roles_account,
        proposal_account,
    };

    role_management::processor::propose(&crate::id(), role_management_accounts, Roles::MINTER)
}

pub(crate) fn process_accept_mintership<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    msg!("Instruction: AcceptInterchainTokenMintership");

    let accounts_iter = &mut accounts.iter();
    let its_config_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let its_config = InterchainTokenService::load(its_config_pda)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    validate_system_account_key(system_account.key)?;
    assert_valid_its_root_pda(its_config_pda, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_config_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account,
        payer,
        resource: token_manager_account,
        destination_user_account,
        destination_roles_account,
        origin_user_account,
        origin_roles_account,
        proposal_account,
    };

    role_management::processor::accept(&crate::id(), role_management_accounts, Roles::MINTER)
}
