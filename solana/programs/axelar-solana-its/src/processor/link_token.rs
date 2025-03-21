//! This module is responsible for functions related to custom token linking

use interchain_token_transfer_gmp::{GMPPayload, LinkToken, RegisterTokenMetadata};
use mpl_token_metadata::accounts::Metadata;
use program_utils::BorshPda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::set_return_data;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use spl_token_2022::state::Mint;

use crate::processor::gmp::GmpAccounts;
use crate::state::token_manager::TokenManager;
use crate::state::{token_manager, InterchainTokenService};
use crate::{assert_valid_its_root_pda, assert_valid_token_manager_pda, FromAccountInfoSlice};

use super::gmp;
use super::token_manager::{DeployTokenManagerAccounts, DeployTokenManagerInternal};

pub(crate) fn process_inbound<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    payload: &LinkToken,
) -> ProgramResult {
    let token_manager_type: token_manager::Type = payload.token_manager_type.try_into()?;
    if token_manager::Type::NativeInterchainToken == token_manager_type {
        return Err(ProgramError::InvalidInstructionData);
    }

    let token_address = Pubkey::new_from_array(
        payload
            .destination_token_address
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidAccountData)?,
    );
    let operator = match payload.link_params.as_ref().try_into() {
        Ok(operator_bytes) => Some(Pubkey::new_from_array(operator_bytes)),
        Err(_err) => None,
    };

    let deploy_token_manager = DeployTokenManagerInternal::new(
        payload.token_manager_type.try_into()?,
        payload.token_id.0,
        token_address,
        operator,
        None,
    );

    let parsed_accounts = DeployTokenManagerAccounts::from_account_info_slice(accounts, &())?;
    let its_root_pda_bump = InterchainTokenService::load(parsed_accounts.its_root_pda)?.bump;

    assert_valid_its_root_pda(
        parsed_accounts.its_root_pda,
        parsed_accounts.gateway_root_pda.key,
        its_root_pda_bump,
    )?;

    let (_, token_manager_pda_bump) =
        crate::find_token_manager_pda(parsed_accounts.its_root_pda.key, payload.token_id.as_ref());

    crate::processor::token_manager::deploy(
        payer,
        &parsed_accounts,
        &deploy_token_manager,
        token_manager_pda_bump,
    )
}

pub(crate) fn process_outbound<'a>(
    accounts: &'a [AccountInfo<'a>],
    salt: [u8; 32],
    destination_chain: String,
    destination_token_address: Vec<u8>,
    token_manager_type: token_manager::Type,
    link_params: Vec<u8>,
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    const OUTBOUND_MESSAGE_ACCOUNTS_IDX: usize = 2;

    let (link_token_accounts, outbound_message_accounts) =
        accounts.split_at(OUTBOUND_MESSAGE_ACCOUNTS_IDX);
    let gmp_accounts = GmpAccounts::from_account_info_slice(outbound_message_accounts, &())?;

    let accounts_iter = &mut link_token_accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;

    msg!("Instruction: ProcessOutbound");
    let token_id = crate::linked_token_id(payer.key, &salt);
    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_token_manager_pda(
        token_manager_account,
        gmp_accounts.its_root_account.key,
        &token_id,
        token_manager.bump,
    )?;

    let message = GMPPayload::LinkToken(LinkToken {
        selector: LinkToken::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_id: token_id.into(),
        token_manager_type: token_manager_type.into(),
        source_token_address: token_manager.token_address.to_bytes().into(),
        destination_token_address: destination_token_address.into(),
        link_params: link_params.into(),
    });

    gmp::process_outbound(
        payer,
        &gmp_accounts,
        &message,
        destination_chain,
        gas_value,
        signing_pda_bump,
        None,
        true,
    )?;

    set_return_data(&token_id);

    Ok(())
}

pub(crate) fn register_token_metadata<'a>(
    accounts: &'a [AccountInfo<'a>],
    gas_value: u64,
    signing_pda_bump: u8,
) -> ProgramResult {
    const OUTBOUND_MESSAGE_ACCOUNTS_IDX: usize = 3;

    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_account = next_account_info(accounts_iter)?;
    let _token_program = next_account_info(accounts_iter)?;

    let (_other, outbound_message_accounts) = accounts.split_at(OUTBOUND_MESSAGE_ACCOUNTS_IDX);
    let gmp_accounts = GmpAccounts::from_account_info_slice(outbound_message_accounts, &())?;
    msg!("Instruction: RegisterTokenMetadata");

    let mint = Mint::unpack(&mint_account.data.borrow())?;
    let payload = GMPPayload::RegisterTokenMetadata(RegisterTokenMetadata {
        selector: RegisterTokenMetadata::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_address: mint_account.key.to_bytes().into(),
        decimals: mint.decimals,
    });

    gmp::process_outbound(
        payer,
        &gmp_accounts,
        &payload,
        crate::ITS_HUB_CHAIN_NAME.to_owned(),
        gas_value,
        signing_pda_bump,
        None,
        false,
    )
}

pub(crate) fn register_custom_token<'a>(
    accounts: &'a [AccountInfo<'a>],
    salt: [u8; 32],
    token_manager_type: token_manager::Type,
    operator: Option<Pubkey>,
) -> ProgramResult {
    register_token(
        accounts,
        &TokenRegistration::Custom {
            salt,
            token_manager_type,
            operator,
        },
    )
}

pub(crate) fn register_canonical_interchain_token<'a>(
    accounts: &'a [AccountInfo<'a>],
) -> ProgramResult {
    msg!("Instruction: RegisterCanonicalInterchainToken");
    register_token(accounts, &TokenRegistration::Canonical)
}

enum TokenRegistration {
    Canonical,
    Custom {
        salt: [u8; 32],
        token_manager_type: token_manager::Type,
        operator: Option<Pubkey>,
    },
}

fn register_token<'a>(
    accounts: &'a [AccountInfo<'a>],
    registration: &TokenRegistration,
) -> ProgramResult {
    const DEPLOY_TOKEN_MANAGER_ACCOUNTS_IDX: usize = 2;

    let (registration_accounts, deploy_token_manager_accounts) =
        accounts.split_at(DEPLOY_TOKEN_MANAGER_ACCOUNTS_IDX);
    let parsed_accounts =
        DeployTokenManagerAccounts::from_account_info_slice(deploy_token_manager_accounts, &())?;
    let (payer, metadata_account) = registration_accounts
        .split_first()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let metadata_account = metadata_account
        .first()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    msg!("Instruction: RegisterToken");
    match Metadata::from_bytes(&metadata_account.try_borrow_data()?) {
        Ok(metadata) => {
            if metadata.mint.ne(parsed_accounts.token_mint.key) {
                msg!("Metadata and mint mismatch");
                return Err(ProgramError::InvalidAccountData);
            }
        }
        Err(error) => {
            msg!("Failed to fetch required Metaplex metadata for the token");

            return Err(error.into());
        }
    };

    let (token_id, token_manager_type, operator) = match *registration {
        TokenRegistration::Canonical => (
            crate::canonical_interchain_token_id(parsed_accounts.token_mint.key),
            token_manager::Type::LockUnlock,
            None,
        ),
        TokenRegistration::Custom {
            salt,
            token_manager_type,
            operator,
        } => (
            crate::linked_token_id(payer.key, &salt),
            token_manager_type,
            operator,
        ),
    };

    let (_, token_manager_pda_bump) =
        crate::find_token_manager_pda(parsed_accounts.its_root_pda.key, &token_id);

    let deploy_token_manager = DeployTokenManagerInternal::new(
        token_manager_type,
        token_id,
        *parsed_accounts.token_mint.key,
        operator,
        None,
    );

    crate::processor::token_manager::deploy(
        payer,
        &parsed_accounts,
        &deploy_token_manager,
        token_manager_pda_bump,
    )?;

    set_return_data(&token_id);

    Ok(())
}
