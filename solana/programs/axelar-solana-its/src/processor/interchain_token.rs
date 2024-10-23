//! Module that handles the processing of the `InterchainToken` deployment.

use alloy_primitives::hex;
use alloy_sol_types::SolValue;
use axelar_message_primitives::U256;
use axelar_rkyv_encoding::types::PublicKey;
use interchain_token_transfer_gmp::DeployInterchainToken;
use program_utils::check_rkyv_initialized_pda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;
use spl_pod::optional_keys::OptionalNonZeroPubkey;
use spl_token_2022::extension::metadata_pointer::instruction::initialize as initialize_metadata_pointer;
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensionsOwned};
use spl_token_2022::instruction::initialize_mint;
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::instruction::{initialize as initialize_metadata, update_field};
use spl_token_metadata_interface::state::{Field, TokenMetadata};

use super::token_manager::DeployTokenManagerInternal;
use crate::instructions::Bumps;
use crate::seed_prefixes;
use crate::state::{token_manager, InterchainTokenService};

const TOKEN_ID_KEY: &str = "token_id";

/// Processes a [`DeployInterchainToken`] GMP message.
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub fn process_deploy<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    payload: DeployInterchainToken,
    bumps: Bumps,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let _system_account = next_account_info(accounts_iter)?;
    let _its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let _token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let _rent_sysvar = next_account_info(accounts_iter)?;
    let additional_minter_account = next_account_info(accounts_iter).ok();

    let token_id = PublicKey::new_ed25519(payload.token_id.0);

    setup_mint(payer, accounts, bumps, payload.decimals, token_id)?;
    setup_metadata(
        payer,
        accounts,
        bumps,
        token_id,
        payload.name,
        payload.symbol,
        String::new(),
    )?;

    let deploy_token_manager = DeployTokenManagerInternal::new(
        token_manager::Type::NativeInterchainToken,
        token_id,
        Some(PublicKey::new_ed25519(token_manager_pda.key.to_bytes())),
        PublicKey::Ed25519(token_mint.key.to_bytes()),
        additional_minter_account.cloned(),
    );

    super::token_manager::deploy(payer, accounts, bumps, deploy_token_manager)?;

    Ok(())
}

/// Processes a request to [`DeployInterchainToken`] on a remote chain.
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub fn process_remote_deploy(
    accounts: &[AccountInfo<'_>],
    payload: &DeployInterchainToken,
    destination_chain: String,
    _gas_value: U256,
) -> ProgramResult {
    // TODO: Make sure destination chain is not solana, if it is, bail.
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let its_root_pda_data = its_root_pda.try_borrow_data()?;
    let its_state = check_rkyv_initialized_pda::<InterchainTokenService>(
        &crate::id(),
        its_root_pda,
        *its_root_pda_data,
    )?;

    // TODO: Get chain's trusted address.
    let destination_address = String::new();

    // TODO: Call gas service to pay gas fee.

    invoke_signed(
        &gateway::instructions::call_contract(
            *gateway_root_pda.key,
            *its_root_pda.key,
            destination_chain,
            destination_address,
            payload.abi_encode_params(),
        )?,
        &[its_root_pda.clone(), gateway_root_pda.clone()],
        &[&[
            seed_prefixes::ITS_SEED,
            gateway_root_pda.key.as_ref(),
            &[its_state.bump],
        ]],
    )?;

    Ok(())
}

fn setup_mint<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    bumps: Bumps,
    decimals: u8,
    token_id: PublicKey,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let rent_sysvar = next_account_info(accounts_iter)?;
    let _minter = next_account_info(accounts_iter).ok();

    let rent = Rent::get()?;
    let account_size =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])?;

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            token_mint.key,
            rent.minimum_balance(account_size).max(1),
            account_size
                .try_into()
                .map_err(|_err| ProgramError::InvalidAccountData)?,
            token_program.key,
        ),
        &[
            payer.clone(),
            token_mint.clone(),
            system_account.clone(),
            token_program.clone(),
            token_manager_pda.clone(),
        ],
        &[&[
            seed_prefixes::INTERCHAIN_TOKEN_SEED,
            its_root_pda.key.as_ref(),
            token_id.as_ref(),
            &[bumps.interchain_token_pda_bump],
        ]],
    )?;

    invoke(
        &initialize_metadata_pointer(
            &spl_token_2022::id(),
            token_mint.key,
            Some(*token_manager_pda.key),
            Some(*token_mint.key),
        )?,
        &[payer.clone(), token_mint.clone(), token_manager_pda.clone()],
    )?;

    invoke(
        &initialize_mint(
            &spl_token_2022::id(),
            token_mint.key,
            token_manager_pda.key,
            Some(token_manager_pda.key),
            decimals,
        )?,
        &[
            token_mint.clone(),
            rent_sysvar.clone(),
            token_manager_pda.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}

fn setup_metadata<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    bumps: Bumps,
    token_id: PublicKey,
    name: String,
    symbol: String,
    uri: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let _rent_sysvar = next_account_info(accounts_iter)?;
    let _minter = next_account_info(accounts_iter).ok();

    let rent = Rent::get()?;
    let (interchain_token_pda, _) = crate::create_interchain_token_pda(
        its_root_pda.key,
        token_id.as_ref(),
        bumps.interchain_token_pda_bump,
    );

    let token_metadata = TokenMetadata {
        update_authority: OptionalNonZeroPubkey(*token_manager_pda.key),
        name,
        symbol,
        uri,
        mint: *token_mint.key,
        additional_metadata: vec![(TOKEN_ID_KEY.to_owned(), hex::encode(token_id.as_ref()))],
    };

    let mint_state =
        StateWithExtensionsOwned::<Mint>::unpack(token_mint.try_borrow_data()?.to_vec())?;
    let account_lamports = token_mint.lamports();
    let new_account_len = mint_state
        .try_get_new_account_len_for_variable_len_extension::<TokenMetadata>(&token_metadata)?;
    let new_rent_exemption_minimum = rent.minimum_balance(new_account_len);
    let additional_lamports = new_rent_exemption_minimum.saturating_sub(account_lamports);

    invoke(
        &system_instruction::transfer(payer.key, token_mint.key, additional_lamports),
        &[payer.clone(), token_mint.clone(), system_account.clone()],
    )?;

    invoke_signed(
        &initialize_metadata(
            &spl_token_2022::id(),
            token_mint.key,
            token_manager_pda.key,
            token_mint.key,
            token_manager_pda.key,
            token_metadata.name,
            token_metadata.symbol,
            token_metadata.uri,
        ),
        &[
            token_mint.clone(),
            token_manager_pda.clone(),
            token_program.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            interchain_token_pda.as_ref(),
            &[bumps.token_manager_pda_bump],
        ]],
    )?;

    invoke_signed(
        &update_field(
            &spl_token_2022::id(),
            token_mint.key,
            token_manager_pda.key,
            Field::Key(TOKEN_ID_KEY.to_owned()),
            hex::encode(token_id.as_ref()),
        ),
        &[
            token_mint.clone(),
            token_manager_pda.clone(),
            token_program.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            interchain_token_pda.as_ref(),
            &[bumps.token_manager_pda_bump],
        ]],
    )?;

    Ok(())
}
