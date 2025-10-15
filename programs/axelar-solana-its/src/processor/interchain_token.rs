//! Module that handles the processing of the `InterchainToken` deployment.

use axelar_solana_gateway::num_traits::Zero;
use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use interchain_token_transfer_gmp::{DeployInterchainToken, GMPPayload};
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1CpiBuilder;
use mpl_token_metadata::types::TokenStandard;
use program_utils::pda::init_pda_raw;
use program_utils::pda::BorshPda;
use program_utils::validate_system_account_key;
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

use super::gmp;
use super::token_manager::DeployTokenManagerInternal;
use crate::accounts::CommonDeployRemoteInterchainTokenAccounts;
use crate::accounts::DeployRemoteCanonicalInterchainTokenAccounts;
use crate::accounts::DeployRemoteInterchainTokenAccounts;
use crate::accounts::DeployRemoteInterchainTokenWithMinterAccounts;
use crate::accounts::{
    CallContractAccounts, DeployInterchainTokenAccounts, DeployTokenManagerAccounts,
};
use crate::state::deploy_approval::DeployApproval;
use crate::state::token_manager::{self, TokenManager};
use crate::state::InterchainTokenService;
use crate::{assert_its_not_paused, assert_valid_deploy_approval_pda, events, find_its_root_pda};
use crate::{assert_valid_its_root_pda, assert_valid_token_manager_pda, seed_prefixes, Roles};
use event_cpi::EventAccounts;

pub(crate) fn process_deploy(
    accounts: DeployInterchainTokenAccounts,
    salt: [u8; 32],
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> ProgramResult {
    let event_accounts_iter = &mut accounts.event_accounts().into_iter();
    event_cpi_accounts!(event_accounts_iter);

    let deploy_salt = crate::interchain_token_deployer_salt(accounts.deployer.key, &salt);
    let token_id = crate::interchain_token_id_internal(&deploy_salt);

    if initial_supply.is_zero() && accounts.minter.is_none() {
        return Err(ProgramError::InvalidArgument);
    }

    if name.len() > mpl_token_metadata::MAX_NAME_LENGTH
        || symbol.len() > mpl_token_metadata::MAX_SYMBOL_LENGTH
    {
        msg!("Name and/or symbol length too long");
        return Err(ProgramError::InvalidArgument);
    }

    emit_cpi!(events::InterchainTokenIdClaimed {
        token_id,
        deployer: *accounts.deployer.key,
        salt: deploy_salt,
    });

    process_inbound_deploy(accounts, token_id, name, symbol, decimals, initial_supply)?;

    set_return_data(&token_id);

    Ok(())
}

pub(crate) fn process_inbound_deploy(
    accounts: DeployInterchainTokenAccounts,
    token_id: [u8; 32],
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
) -> ProgramResult {
    msg!("Instruction: InboundDeploy");

    let event_accounts_iter = &mut accounts.event_accounts().into_iter();
    event_cpi_accounts!(event_accounts_iter);

    let its_config = InterchainTokenService::load(accounts.its_root)?;
    assert_valid_its_root_pda(accounts.its_root, its_config.bump)?;
    assert_its_not_paused(&its_config)?;

    let (interchain_token_pda, interchain_token_pda_bump) =
        crate::find_interchain_token_pda(accounts.its_root.key, &token_id);
    if interchain_token_pda.ne(accounts.mint.key) {
        msg!("Invalid mint account provided");
        return Err(ProgramError::InvalidArgument);
    }

    let (token_manager_pda, token_manager_pda_bump) =
        crate::find_token_manager_pda(accounts.its_root.key, &token_id);
    if token_manager_pda.ne(accounts.token_manager.key) {
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
        *accounts.mint.key,
        accounts.minter.map(|account| *account.key),
        accounts.minter.map(|account| *account.key),
    );

    let deploy_token_manager_accounts = DeployTokenManagerAccounts::from(accounts);
    super::token_manager::deploy(
        &deploy_token_manager_accounts,
        &deploy_token_manager,
        token_manager_pda_bump,
    )?;

    emit_cpi!(events::InterchainTokenDeployed {
        token_id,
        token_address: *deploy_token_manager_accounts.mint.key,
        minter: deploy_token_manager_accounts
            .operator
            .map(|account| *account.key)
            .unwrap_or_default(),
        name: truncated_name,
        symbol: truncated_symbol,
        decimals,
    });

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

pub(crate) fn process_outbound_deploy(
    accounts: CommonDeployRemoteInterchainTokenAccounts,
    token_id: &[u8; 32],
    destination_chain: String,
    maybe_destination_minter: Option<Vec<u8>>,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    msg!("Instruction: OutboundDeploy");

    // Get metadata with fallback logic (Token 2022 extensions first, then Metaplex)
    let (name, symbol) = get_token_metadata(accounts.mint, Some(accounts.mpl_token_metadata))?;
    let mint_data_ref = accounts.mint.try_borrow_data()?;
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data_ref)?;
    let mint_data = mint_state.base;

    let token_manager = TokenManager::load(accounts.token_manager)?;
    assert_valid_token_manager_pda(
        accounts.token_manager,
        accounts.its_root.key,
        token_id,
        token_manager.bump,
    )?;
    if token_manager.token_address != *accounts.mint.key {
        msg!("TokenManager doesn't match mint");
        return Err(ProgramError::InvalidArgument);
    }

    let deployment_started_events = events::InterchainTokenDeploymentStarted {
        token_id: token_id.to_owned(),
        token_name: name,
        token_symbol: symbol,
        token_decimals: mint_data.decimals,
        minter: maybe_destination_minter.clone().unwrap_or_default(),
        destination_chain: destination_chain.clone(),
    };

    let event_accounts_iter = &mut accounts.event_accounts().into_iter();
    event_cpi_accounts!(event_accounts_iter);
    emit_cpi!(deployment_started_events);

    let message = GMPPayload::DeployInterchainToken(DeployInterchainToken {
        selector: DeployInterchainToken::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_id: token_id.into(),
        name: deployment_started_events.token_name,
        symbol: deployment_started_events.token_symbol,
        decimals: mint_data.decimals,
        minter: maybe_destination_minter.unwrap_or_default().into(),
    });

    let gmp_accounts = CallContractAccounts::try_from(accounts)?;
    let its_root_config = InterchainTokenService::load(gmp_accounts.its_root)?;
    assert_valid_its_root_pda(gmp_accounts.its_root, its_root_config.bump)?;
    if destination_chain == its_root_config.chain_name {
        msg!("Cannot deploy remotely to the origin chain");
        return Err(ProgramError::InvalidInstructionData);
    }

    gmp::process_call_contract(
        &gmp_accounts,
        &message,
        destination_chain.clone(),
        gas_value,
        signing_pda_bump,
        true,
    )?;

    Ok(())
}

pub(crate) fn deploy_remote_interchain_token(
    accounts: DeployRemoteInterchainTokenAccounts,
    salt: [u8; 32],
    destination_chain: String,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    let deploy_salt = crate::interchain_token_deployer_salt(accounts.deployer.key, &salt);
    let token_id = crate::interchain_token_id_internal(&deploy_salt);

    process_outbound_deploy(
        accounts.try_into()?,
        &token_id,
        destination_chain,
        None,
        gas_value,
        signing_pda_bump,
    )
}

pub(crate) fn deploy_remote_interchain_token_with_minter(
    accounts: DeployRemoteInterchainTokenWithMinterAccounts,
    salt: [u8; 32],
    destination_chain: String,
    destination_minter: Vec<u8>,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    ensure_roles(
        &crate::id(),
        accounts.token_manager,
        accounts.minter,
        accounts.minter_roles,
        Roles::MINTER,
    )?;

    let deploy_salt = crate::interchain_token_deployer_salt(accounts.deployer.key, &salt);
    let token_id = crate::interchain_token_id_internal(&deploy_salt);

    process_outbound_deploy(
        accounts.clone().try_into()?,
        &token_id,
        destination_chain.clone(),
        Some(destination_minter.clone()),
        gas_value,
        signing_pda_bump,
    )?;

    // This closes the account and transfers lamports back, thus, this needs to happen after all
    // CPIs
    use_deploy_approval(
        accounts.payer,
        accounts.minter,
        accounts.deployment_approval,
        &destination_minter,
        &token_id,
        &destination_chain,
    )?;

    Ok(())
}

pub(crate) fn deploy_remote_canonical_interchain_token(
    accounts: DeployRemoteCanonicalInterchainTokenAccounts,
    destination_chain: String,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    let deploy_salt = crate::canonical_interchain_token_deploy_salt(accounts.mint.key);
    let token_id = crate::interchain_token_id_internal(&deploy_salt);

    process_outbound_deploy(
        accounts,
        &token_id,
        destination_chain,
        None,
        gas_value,
        signing_pda_bump,
    )
}

pub(crate) fn process_mint<'a>(accounts: &'a [AccountInfo<'a>], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let mint_account = next_account_info(accounts_iter)?;
    let destination_account = next_account_info(accounts_iter)?;
    let its_root_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let minter_account = next_account_info(accounts_iter)?;
    let minter_roles_account = next_account_info(accounts_iter)?;
    let token_program_account = next_account_info(accounts_iter)?;

    msg!("Instruction: MintInterchainToken");

    let its_root_config = InterchainTokenService::load(its_root_account)?;
    assert_valid_its_root_pda(its_root_account, its_root_config.bump)?;

    let token_manager = TokenManager::load(token_manager_account)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_root_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    if token_manager.token_address.as_ref() != mint_account.key.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    spl_token_2022::check_spl_token_program_account(token_program_account.key)?;

    if mint_account.owner != token_program_account.key {
        return Err(ProgramError::IncorrectProgramId);
    }

    ensure_signer_roles(
        &crate::id(),
        token_manager_account,
        minter_account,
        minter_roles_account,
        Roles::MINTER,
    )?;

    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            token_program_account.key,
            mint_account.key,
            destination_account.key,
            token_manager_account.key,
            &[],
            amount,
        )?,
        &[
            mint_account.clone(),
            destination_account.clone(),
            token_manager_account.clone(),
            token_program_account.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_account.key.as_ref(),
            token_manager.token_id.as_ref(),
            &[token_manager.bump],
        ]],
    )?;
    Ok(())
}

fn setup_mint(
    accounts: &DeployInterchainTokenAccounts,
    decimals: u8,
    token_id: &[u8],
    interchain_token_pda_bump: u8,
    token_manager_pda_bump: u8,
    initial_supply: u64,
) -> ProgramResult {
    init_pda_raw(
        accounts.payer,
        accounts.mint,
        accounts.token_program.key,
        accounts.system_program,
        spl_token_2022::state::Mint::LEN
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        &[
            seed_prefixes::INTERCHAIN_TOKEN_SEED,
            accounts.its_root.key.as_ref(),
            token_id,
            &[interchain_token_pda_bump],
        ],
    )?;

    invoke(
        &initialize_mint(
            &spl_token_2022::id(),
            accounts.mint.key,
            accounts.token_manager.key,
            Some(accounts.token_manager.key),
            decimals,
        )?,
        &[
            accounts.mint.clone(),
            accounts.rent_sysvar.clone(),
            accounts.token_manager.clone(),
            accounts.token_program.clone(),
        ],
    )?;

    if initial_supply > 0 {
        crate::create_associated_token_account_idempotent(
            accounts.payer,
            accounts.mint,
            accounts.deployer_ata,
            accounts.deployer,
            accounts.system_program,
            accounts.token_program,
        )?;

        invoke_signed(
            &spl_token_2022::instruction::mint_to(
                accounts.token_program.key,
                accounts.mint.key,
                accounts.deployer_ata.key,
                accounts.token_manager.key,
                &[],
                initial_supply,
            )?,
            &[
                accounts.payer.clone(),
                accounts.deployer.clone(),
                accounts.mint.clone(),
                accounts.deployer_ata.clone(),
                accounts.token_manager.clone(),
                accounts.token_program.clone(),
            ],
            &[&[
                seed_prefixes::TOKEN_MANAGER_SEED,
                accounts.its_root.key.as_ref(),
                token_id,
                &[token_manager_pda_bump],
            ]],
        )?;
    }

    Ok(())
}

fn setup_metadata(
    accounts: &DeployInterchainTokenAccounts<'_>,
    token_id: &[u8],
    name: String,
    symbol: String,
    uri: String,
    token_manager_pda_bump: u8,
) -> ProgramResult {
    CreateV1CpiBuilder::new(accounts.mpl_token_metadata_program)
        .metadata(accounts.mpl_token_metadata)
        .token_standard(TokenStandard::Fungible)
        .mint(accounts.mint, false)
        .authority(accounts.token_manager)
        .update_authority(accounts.token_manager, true)
        .payer(accounts.payer)
        .is_mutable(false)
        .name(name)
        .symbol(symbol)
        .uri(uri)
        .seller_fee_basis_points(0)
        .system_program(accounts.system_program)
        .sysvar_instructions(accounts.sysvar_instructions)
        .invoke_signed(&[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            accounts.its_root.key.as_ref(),
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

    let payer_account = next_account_info(accounts_iter)?;
    let minter_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let minter_roles_account = next_account_info(accounts_iter)?;
    let deploy_approval_account = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;
    event_cpi_accounts!(accounts_iter);

    validate_system_account_key(system_program_account.key)?;
    msg!("Instruction: ApproveDeployRemoteInterchainToken");

    if !payer_account.is_signer {
        msg!("Payer should be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // This also ensures minter.is_signer == true
    ensure_signer_roles(
        &crate::id(),
        token_manager_account,
        minter_account,
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
        crate::find_deployment_approval_pda(minter_account.key, &token_id, &destination_chain);
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
        system_program_account,
        payer_account,
        deploy_approval_account,
        &[
            seed_prefixes::DEPLOYMENT_APPROVAL_SEED,
            minter_account.key.as_ref(),
            &token_id,
            destination_chain_hash.as_ref(),
            &[bump],
        ],
    )?;

    emit_cpi!(events::DeployRemoteInterchainTokenApproval {
        minter: *minter_account.key,
        deployer,
        token_id,
        destination_chain,
        destination_minter,
    });

    Ok(())
}

pub(crate) fn revoke_deploy_remote_interchain_token(
    accounts: &[AccountInfo<'_>],
    deployer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer_account = next_account_info(accounts_iter)?;
    let minter_account = next_account_info(accounts_iter)?;
    let deploy_approval_account = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;
    event_cpi_accounts!(accounts_iter);

    validate_system_account_key(system_program_account.key)?;
    msg!("Instruction: RevokeDeployRemoteInterchainToken");

    if !payer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let token_id = crate::interchain_token_id(&deployer, &salt);
    let approval = DeployApproval::load(deploy_approval_account)?;

    assert_valid_deploy_approval_pda(
        deploy_approval_account,
        minter_account.key,
        &token_id,
        &destination_chain,
        approval.bump,
    )?;

    emit_cpi!(events::RevokeRemoteInterchainTokenApproval {
        minter: *minter_account.key,
        deployer,
        token_id,
        destination_chain,
    });

    program_utils::pda::close_pda(payer_account, deploy_approval_account, &crate::id())
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
    let its_root_account = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let sender_user_account = next_account_info(accounts_iter)?;
    let sender_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_program_account.key)?;

    if sender_user_account.key == destination_user_account.key {
        msg!("Source and destination accounts are the same");
        return Err(ProgramError::InvalidArgument);
    }

    let its_config = InterchainTokenService::load(its_root_account)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_its_root_pda(its_root_account, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_root_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let role_add_accounts = RoleAddAccounts {
        system_account: system_program_account,
        payer: payer_account,
        authority_user_account: sender_user_account,
        authority_roles_account: sender_roles_account,
        resource: token_manager_account,
        target_user_account: destination_user_account,
        target_roles_account: destination_roles_account,
    };

    let role_remove_accounts = RoleRemoveAccounts {
        system_account: system_program_account,
        payer: payer_account,
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
    let its_root_account = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_program_account.key)?;

    let its_config = InterchainTokenService::load(its_root_account)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_its_root_pda(its_root_account, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_root_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account: system_program_account,
        payer: payer_account,
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
    let its_root_account = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let its_config = InterchainTokenService::load(its_root_account)?;
    let token_manager = TokenManager::load(token_manager_account)?;

    validate_system_account_key(system_program_account.key)?;
    assert_valid_its_root_pda(its_root_account, its_config.bump)?;
    assert_valid_token_manager_pda(
        token_manager_account,
        its_root_account.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account: system_program_account,
        payer: payer_account,
        resource: token_manager_account,
        destination_user_account,
        destination_roles_account,
        origin_user_account,
        origin_roles_account,
        proposal_account,
    };

    role_management::processor::accept(&crate::id(), role_management_accounts, Roles::MINTER)
}
