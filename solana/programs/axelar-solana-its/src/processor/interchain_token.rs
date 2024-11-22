//! Module that handles the processing of the `InterchainToken` deployment.

use alloy_primitives::hex;
use interchain_token_transfer_gmp::DeployInterchainToken;
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
use super::LocalAction;
use crate::instructions::Bumps;
use crate::seed_prefixes;
use crate::state::token_manager;

const TOKEN_ID_KEY: &str = "token_id";

impl LocalAction for DeployInterchainToken {
    fn process_local_action<'a>(
        self,
        payer: &AccountInfo<'a>,
        accounts: &[AccountInfo<'a>],
        bumps: Bumps,
    ) -> ProgramResult {
        process_deploy(payer, accounts, self, bumps)
    }
}

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
    let _token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let _token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let _its_roles_pda = next_account_info(accounts_iter)?;
    let _rent_sysvar = next_account_info(accounts_iter)?;
    let additional_minter = next_account_info(accounts_iter).ok();
    let _additional_minter_roles_pda = next_account_info(accounts_iter).ok();

    setup_mint(
        payer,
        accounts,
        bumps,
        payload.decimals,
        &payload.token_id.0,
    )?;
    setup_metadata(
        payer,
        accounts,
        bumps,
        &payload.token_id.0,
        payload.name,
        payload.symbol,
        String::new(),
    )?;

    // The minter passed in the DeployInterchainToken call is used as the
    // `TokenManager` operator as well, see:
    // https://github.com/axelarnetwork/interchain-token-service/blob/v2.0.1/contracts/InterchainTokenService.sol#L758
    let deploy_token_manager = DeployTokenManagerInternal::new(
        token_manager::Type::NativeInterchainToken,
        payload.token_id.0,
        *token_mint.key,
        additional_minter.map(|account| *account.key),
        additional_minter.map(|account| *account.key),
    );

    super::token_manager::deploy(payer, accounts, bumps, &deploy_token_manager)?;

    Ok(())
}

fn setup_mint<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    bumps: Bumps,
    decimals: u8,
    token_id: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let _its_roles_pda = next_account_info(accounts_iter)?;
    let rent_sysvar = next_account_info(accounts_iter)?;
    let _minter = next_account_info(accounts_iter).ok();
    let _minter_roles_pda = next_account_info(accounts_iter).ok();

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
            token_id,
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
    token_id: &[u8],
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
    let _its_roles_pda = next_account_info(accounts_iter)?;
    let _rent_sysvar = next_account_info(accounts_iter)?;
    let _minter = next_account_info(accounts_iter).ok();
    let _minter_roles_pda = next_account_info(accounts_iter).ok();

    let rent = Rent::get()?;
    let (interchain_token_pda, _) = crate::create_interchain_token_pda(
        its_root_pda.key,
        token_id,
        bumps.interchain_token_pda_bump,
    );

    let token_metadata = TokenMetadata {
        update_authority: OptionalNonZeroPubkey(*token_manager_pda.key),
        name,
        symbol,
        uri,
        mint: *token_mint.key,
        additional_metadata: vec![(TOKEN_ID_KEY.to_owned(), hex::encode(token_id))],
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
            hex::encode(token_id),
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
