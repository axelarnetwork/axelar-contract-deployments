//! This module is responsible for functions related to custom token linking

use event_utils::Event as _;
use interchain_token_transfer_gmp::{GMPPayload, LinkToken, RegisterTokenMetadata};
use program_utils::pda::BorshPda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::set_return_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions};
use spl_token_2022::state::Mint;

use crate::processor::gmp::{self, GmpAccounts};
use crate::processor::interchain_token;
use crate::processor::token_manager::{DeployTokenManagerAccounts, DeployTokenManagerInternal};
use crate::state::token_manager::TokenManager;
use crate::state::{token_manager, InterchainTokenService};
use crate::{
    assert_its_not_paused, assert_valid_its_root_pda, assert_valid_token_manager_pda, event,
    FromAccountInfoSlice,
};

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

    let parsed_accounts =
        DeployTokenManagerAccounts::from_account_info_slice(accounts, &Some(payer))?;
    let its_root_pda_bump = InterchainTokenService::load(parsed_accounts.its_root_pda)?.bump;

    assert_valid_its_root_pda(parsed_accounts.its_root_pda, its_root_pda_bump)?;

    let (_, token_manager_pda_bump) =
        crate::find_token_manager_pda(parsed_accounts.its_root_pda.key, payload.token_id.as_ref());

    crate::processor::token_manager::deploy(
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

    let its_root_config = InterchainTokenService::load(gmp_accounts.its_root_account)?;
    assert_valid_its_root_pda(gmp_accounts.its_root_account, its_root_config.bump)?;
    if destination_chain == its_root_config.chain_name {
        msg!("Cannot link to another token on the same chain");
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts_iter = &mut link_token_accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;

    if !payer.is_signer {
        msg!("Payer Should be Signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    msg!("Instruction: ProcessOutbound");
    let deploy_salt = crate::linked_token_deployer_salt(payer.key, &salt);
    let token_id = crate::interchain_token_id_internal(&deploy_salt);

    event::InterchainTokenIdClaimed {
        token_id,
        deployer: *payer.key,
        salt: deploy_salt,
    }
    .emit();

    let token_manager = TokenManager::load(token_manager_account)?;

    assert_valid_token_manager_pda(
        token_manager_account,
        gmp_accounts.its_root_account.key,
        &token_id,
        token_manager.bump,
    )?;

    let link_started_event = event::LinkTokenStarted {
        token_id,
        destination_chain,
        source_token_address: token_manager.token_address,
        destination_token_address,
        token_manager_type: token_manager_type.into(),
        params: link_params,
    };
    link_started_event.emit();

    let message = GMPPayload::LinkToken(LinkToken {
        selector: LinkToken::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_id: token_id.into(),
        token_manager_type: token_manager_type.into(),
        source_token_address: token_manager.token_address.to_bytes().into(),
        destination_token_address: link_started_event.destination_token_address.into(),
        link_params: link_started_event.params.into(),
    });

    gmp::process_outbound(
        payer,
        &gmp_accounts,
        &message,
        link_started_event.destination_chain,
        gas_value,
        signing_pda_bump,
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
    const OUTBOUND_MESSAGE_ACCOUNTS_IDX: usize = 2;

    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_account = next_account_info(accounts_iter)?;

    let (_other, outbound_message_accounts) = accounts.split_at(OUTBOUND_MESSAGE_ACCOUNTS_IDX);
    let gmp_accounts = GmpAccounts::from_account_info_slice(outbound_message_accounts, &())?;
    msg!("Instruction: RegisterTokenMetadata");

    let mint_data = mint_account.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    let payload = GMPPayload::RegisterTokenMetadata(RegisterTokenMetadata {
        selector: RegisterTokenMetadata::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_address: mint_account.key.to_bytes().into(),
        decimals: mint.base.decimals,
    });

    event::TokenMetadataRegistered {
        token_address: *mint_account.key,
        decimals: mint.base.decimals,
    }
    .emit();

    gmp::process_outbound(
        payer,
        &gmp_accounts,
        &payload,
        crate::ITS_HUB_CHAIN_NAME.to_owned(),
        gas_value,
        signing_pda_bump,
        false,
    )
}

pub(crate) fn register_custom_token<'a>(
    accounts: &'a [AccountInfo<'a>],
    salt: [u8; 32],
    token_manager_type: token_manager::Type,
    operator: Option<Pubkey>,
) -> ProgramResult {
    if token_manager_type == token_manager::Type::NativeInterchainToken {
        return Err(ProgramError::InvalidInstructionData);
    }

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

    let (payer, metadata_account) = registration_accounts
        .split_first()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let maybe_metadata_account = metadata_account.first();
    let parsed_accounts = DeployTokenManagerAccounts::from_account_info_slice(
        deploy_token_manager_accounts,
        &Some(payer),
    )?;

    msg!("Instruction: RegisterToken");

    let its_config = InterchainTokenService::load(parsed_accounts.its_root_pda)?;
    assert_valid_its_root_pda(parsed_accounts.its_root_pda, its_config.bump)?;
    assert_its_not_paused(&its_config)?;
    let mint_data = parsed_accounts.token_mint.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    let has_fee_extension = mint
        .get_extension_types()?
        .contains(&ExtensionType::TransferFeeConfig);

    let (token_manager_type, operator, deploy_salt) = match *registration {
        TokenRegistration::Canonical => {
            // Metadata is required for canonical tokens
            if let Err(_err) = interchain_token::get_token_metadata(
                parsed_accounts.token_mint,
                maybe_metadata_account,
            ) {
                return Err(ProgramError::InvalidAccountData);
            }

            let token_manager_type = if has_fee_extension {
                token_manager::Type::LockUnlockFee
            } else {
                token_manager::Type::LockUnlock
            };

            (
                token_manager_type,
                None,
                crate::canonical_interchain_token_deploy_salt(parsed_accounts.token_mint.key),
            )
        }
        TokenRegistration::Custom {
            salt,
            token_manager_type,
            operator,
        } => (
            token_manager_type,
            operator,
            crate::linked_token_deployer_salt(payer.key, &salt),
        ),
    };

    let token_id = crate::interchain_token_id_internal(&deploy_salt);
    let (_, token_manager_pda_bump) =
        crate::find_token_manager_pda(parsed_accounts.its_root_pda.key, &token_id);
    crate::assert_valid_token_manager_pda(
        parsed_accounts.token_manager_pda,
        parsed_accounts.its_root_pda.key,
        &token_id,
        token_manager_pda_bump,
    )?;

    event::InterchainTokenIdClaimed {
        token_id,
        deployer: *payer.key,
        salt: deploy_salt,
    }
    .emit();

    let deploy_token_manager = DeployTokenManagerInternal::new(
        token_manager_type,
        token_id,
        *parsed_accounts.token_mint.key,
        operator,
        None,
    );

    crate::processor::token_manager::deploy(
        &parsed_accounts,
        &deploy_token_manager,
        token_manager_pda_bump,
    )?;

    set_return_data(&token_id);

    Ok(())
}
