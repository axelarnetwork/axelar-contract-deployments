use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::log::sol_log_data;
use solana_program::program::invoke;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use crate::event_utils::{parse_u64_le, read_array, read_string, EventParseError};
use crate::state::Config;
use crate::{assert_valid_config_pda, event_prefixes, seed_prefixes};

fn ensure_valid_config_pda_ata(
    config_pda_ata: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    config_pda: &AccountInfo<'_>,
) -> ProgramResult {
    if config_pda_ata.owner != token_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }
    let ata_data =
        spl_token_2022::state::Account::unpack_from_slice(&config_pda_ata.try_borrow_data()?)?;
    if ata_data.mint != *mint.key || ata_data.owner != *config_pda.key {
        return Err(ProgramError::InvalidAccountData);
    };
    Ok(())
}

fn ensure_valid_config_pda(config_pda: &AccountInfo<'_>, program_id: &Pubkey) -> ProgramResult {
    config_pda.check_initialized_pda_without_deserialization(program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    assert_valid_config_pda(config.bump, &config.salt, &config.authority, config_pda.key)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn transfer_tokens(
    token_program: &AccountInfo<'_>,
    sender_ata: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    receiver_ata: &AccountInfo<'_>,
    sender_authority: &AccountInfo<'_>,
    signer_pubkeys: &[AccountInfo<'_>],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    spl_token_2022::instruction::transfer_checked(
        token_program.key,
        sender_ata.key,
        mint.key,
        receiver_ata.key,
        sender_authority.key,
        signer_pubkeys
            .iter()
            .map(|x| x.key)
            .collect::<Vec<_>>()
            .as_slice(),
        amount,
        decimals,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_pay_spl_for_contract_call(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    params: &[u8],
    gas_fee_amount: u64,
    decimals: u8,
) -> ProgramResult {
    let (accounts, signer_pubkeys) = accounts.split_at(6);
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let sender_ata = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        sender_ata,
        mint,
        config_pda_ata,
        sender,
        signer_pubkeys,
        gas_fee_amount,
        decimals,
    )?;

    invoke(
        &ix,
        &[
            sender.clone(),
            mint.clone(),
            sender_ata.clone(),
            config_pda_ata.clone(),
            token_program.clone(),
        ],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::SPL_PAID_FOR_CONTRACT_CALL,
        &config_pda.key.to_bytes(),
        &config_pda_ata.key.to_bytes(),
        &mint.key.to_bytes(),
        &token_program.key.to_bytes(),
        &destination_chain.into_bytes(),
        &destination_address.into_bytes(),
        &payload_hash,
        &refund_address.to_bytes(),
        params,
        &gas_fee_amount.to_le_bytes(),
    ]);

    Ok(())
}

pub(crate) fn add_spl_gas(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    log_index: u64,
    gas_fee_amount: u64,
    refund_address: Pubkey,
    decimals: u8,
) -> ProgramResult {
    let (accounts, signer_pubkeys) = accounts.split_at(6);
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let sender_ata = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        sender_ata,
        mint,
        config_pda_ata,
        sender,
        signer_pubkeys,
        gas_fee_amount,
        decimals,
    )?;

    invoke(
        &ix,
        &[
            sender.clone(),
            mint.clone(),
            sender_ata.clone(),
            config_pda_ata.clone(),
            token_program.clone(),
        ],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::SPL_GAS_ADDED,
        &config_pda.key.to_bytes(),
        &config_pda_ata.key.to_bytes(),
        &mint.key.to_bytes(),
        &token_program.key.to_bytes(),
        &tx_hash,
        &log_index.to_le_bytes(),
        &refund_address.to_bytes(),
        &gas_fee_amount.to_le_bytes(),
    ]);

    Ok(())
}

pub(crate) fn collect_fees_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let authority = next_account_info(accounts)?;
    let receiver_account = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    // Check: Authority mtaches
    if authority.key != &config.authority {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Check: Authority is signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        config_pda_ata,
        mint,
        receiver_account,
        config_pda,
        &[],
        amount,
        decimals,
    )?;

    invoke_signed(
        &ix,
        &[
            config_pda.clone(),
            mint.clone(),
            config_pda_ata.clone(),
            receiver_account.clone(),
            token_program.clone(),
        ],
        &[&[
            seed_prefixes::CONFIG_SEED,
            &config.salt,
            config.authority.as_ref(),
            &[config.bump],
        ]],
    )?;

    Ok(())
}

pub(crate) fn refund_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    log_index: u64,
    fees: u64,
    decimals: u8,
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let authority = next_account_info(accounts)?;
    let receiver_account = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_ata = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    // Check: Authority mtaches
    if authority.key != &config.authority {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Check: Authority is signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_ata is owned by the Token Program and matches expected fields
    ensure_valid_config_pda_ata(config_pda_ata, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        config_pda_ata,
        mint,
        receiver_account,
        config_pda,
        &[],
        fees,
        decimals,
    )?;

    invoke_signed(
        &ix,
        &[
            config_pda.clone(),
            mint.clone(),
            config_pda_ata.clone(),
            receiver_account.clone(),
            token_program.clone(),
        ],
        &[&[
            seed_prefixes::CONFIG_SEED,
            &config.salt,
            config.authority.as_ref(),
            &[config.bump],
        ]],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::SPL_GAS_REFUNDED,
        &tx_hash,
        &config_pda.key.to_bytes(),
        &config_pda_ata.key.to_bytes(),
        &mint.key.to_bytes(),
        &token_program.key.to_bytes(),
        &log_index.to_le_bytes(),
        &receiver_account.key.to_bytes(),
        &fees.to_le_bytes(),
    ]);

    Ok(())
}

/// Represents the event emitted when native gas is paid for a contract call.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SplGasPaidForContractCallEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// The Gas service config associated token account PDA
    pub config_pda_ata: Pubkey,
    /// Mint of the token
    pub mint: Pubkey,
    /// The token program id
    pub token_program_id: Pubkey,
    /// Destination chain on the Axelar network
    pub destination_chain: String,
    /// Destination address on the Axelar network
    pub destination_address: String,
    /// The payload hash for the event we're paying for
    pub payload_hash: [u8; 32],
    /// The refund address
    pub refund_address: Pubkey,
    /// Extra parameters to be passed
    pub params: Vec<u8>,
    /// The amount of SOL to send
    pub gas_fee_amount: u64,
}

impl SplGasPaidForContractCallEvent {
    /// Construct a new event from byte slices
    ///
    /// # Errors
    /// - if the data could not be parsed into an event
    pub fn new<I: Iterator<Item = Vec<u8>>>(mut data: I) -> Result<Self, EventParseError> {
        let config_pda_data = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda"))?;
        let config_pda = Pubkey::new_from_array(read_array::<32>("config_pda", &config_pda_data)?);

        let config_pda_ata = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda_ata"))?;
        let config_pda_ata =
            Pubkey::new_from_array(read_array::<32>("config_pda_ata", &config_pda_ata)?);

        let mint = data.next().ok_or(EventParseError::MissingData("mint"))?;
        let mint = Pubkey::new_from_array(read_array::<32>("mint", &mint)?);

        let token_program_id = data
            .next()
            .ok_or(EventParseError::MissingData("token_program_id"))?;
        let token_program_id =
            Pubkey::new_from_array(read_array::<32>("token_program_id", &token_program_id)?);

        let destination_chain_data = data
            .next()
            .ok_or(EventParseError::MissingData("destination_chain"))?;
        let destination_chain = read_string("destination_chain", destination_chain_data)?;

        let destination_address_data = data
            .next()
            .ok_or(EventParseError::MissingData("destination_address"))?;
        let destination_address = read_string("destination_address", destination_address_data)?;

        let payload_hash_data = data
            .next()
            .ok_or(EventParseError::MissingData("payload_hash"))?;
        let payload_hash = read_array::<32>("payload_hash", &payload_hash_data)?;

        let refund_address_data = data
            .next()
            .ok_or(EventParseError::MissingData("refund_address"))?;
        let refund_address =
            Pubkey::new_from_array(read_array::<32>("refund_address", &refund_address_data)?);

        let params = data.next().ok_or(EventParseError::MissingData("params"))?;

        let gas_fee_amount_data = data
            .next()
            .ok_or(EventParseError::MissingData("gas_fee_amount"))?;
        let gas_fee_amount = parse_u64_le("gas_fee_amount", &gas_fee_amount_data)?;

        Ok(Self {
            config_pda,
            config_pda_ata,
            mint,
            token_program_id,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            params,
            gas_fee_amount,
        })
    }
}

/// Represents the event emitted when native gas is added.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SplGasAddedEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// The Gas service config associated token account PDA
    pub config_pda_ata: Pubkey,
    /// Mint of the token
    pub mint: Pubkey,
    /// The token program id
    pub token_program_id: Pubkey,
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// index of the log
    pub log_index: u64,
    /// The refund address
    pub refund_address: Pubkey,
    /// amount of SOL
    pub gas_fee_amount: u64,
}

impl SplGasAddedEvent {
    /// Construct a new event from byte slices
    ///
    /// # Errors
    /// - if the data could not be parsed into an event
    pub fn new<I: Iterator<Item = Vec<u8>>>(mut data: I) -> Result<Self, EventParseError> {
        let config_pda = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda"))?;
        let config_pda = Pubkey::new_from_array(read_array::<32>("config_pda", &config_pda)?);

        let config_pda_ata = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda_ata"))?;
        let config_pda_ata =
            Pubkey::new_from_array(read_array::<32>("config_pda_ata", &config_pda_ata)?);

        let mint = data.next().ok_or(EventParseError::MissingData("mint"))?;
        let mint = Pubkey::new_from_array(read_array::<32>("mint", &mint)?);

        let token_program_id = data
            .next()
            .ok_or(EventParseError::MissingData("token_program_id"))?;
        let token_program_id =
            Pubkey::new_from_array(read_array::<32>("token_program_id", &token_program_id)?);

        let tx_hash_data = data.next().ok_or(EventParseError::MissingData("tx_hash"))?;
        let tx_hash = read_array::<64>("tx_hash", &tx_hash_data)?;

        let log_index_data = data
            .next()
            .ok_or(EventParseError::MissingData("log_index"))?;
        let log_index = parse_u64_le("log_index", &log_index_data)?;

        let refund_address_data = data
            .next()
            .ok_or(EventParseError::MissingData("refund_address"))?;
        let refund_address =
            Pubkey::new_from_array(read_array::<32>("refund_address", &refund_address_data)?);

        let gas_fee_amount_data = data
            .next()
            .ok_or(EventParseError::MissingData("gas_fee_amount"))?;
        let gas_fee_amount = parse_u64_le("gas_fee_amount", &gas_fee_amount_data)?;

        Ok(Self {
            config_pda,
            config_pda_ata,
            mint,
            token_program_id,
            tx_hash,
            log_index,
            refund_address,
            gas_fee_amount,
        })
    }
}

/// Represents the event emitted when native gas is refunded.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SplGasRefundedEvent {
    /// The Gas service config associated token account PDA
    pub config_pda_ata: Pubkey,
    /// Mint of the token
    pub mint: Pubkey,
    /// The token program id
    pub token_program_id: Pubkey,
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// The log index
    pub log_index: u64,
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// amount of SOL
    pub fees: u64,
}

impl SplGasRefundedEvent {
    /// Construct a new event from byte slices
    ///
    /// # Errors
    /// - if the data could not be parsed into an event
    pub fn new<I: Iterator<Item = Vec<u8>>>(mut data: I) -> Result<Self, EventParseError> {
        let tx_hash_data = data.next().ok_or(EventParseError::MissingData("tx_hash"))?;
        let tx_hash = read_array::<64>("tx_hash", &tx_hash_data)?;

        let config_pda_data = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda"))?;
        let config_pda = Pubkey::new_from_array(read_array::<32>("config_pda", &config_pda_data)?);

        let config_pda_ata = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda_ata"))?;
        let config_pda_ata =
            Pubkey::new_from_array(read_array::<32>("config_pda_ata", &config_pda_ata)?);

        let mint = data.next().ok_or(EventParseError::MissingData("mint"))?;
        let mint = Pubkey::new_from_array(read_array::<32>("mint", &mint)?);

        let token_program_id = data
            .next()
            .ok_or(EventParseError::MissingData("token_program_id"))?;
        let token_program_id =
            Pubkey::new_from_array(read_array::<32>("token_program_id", &token_program_id)?);

        let log_index_data = data
            .next()
            .ok_or(EventParseError::MissingData("log_index"))?;
        let log_index = parse_u64_le("log_index", &log_index_data)?;

        let receiver_data = data
            .next()
            .ok_or(EventParseError::MissingData("receiver"))?;
        let receiver = Pubkey::new_from_array(read_array::<32>("receiver", &receiver_data)?);

        let fees_data = data.next().ok_or(EventParseError::MissingData("fees"))?;
        let fees = parse_u64_le("fees", &fees_data)?;

        Ok(Self {
            config_pda_ata,
            mint,
            token_program_id,
            tx_hash,
            config_pda,
            log_index,
            receiver,
            fees,
        })
    }
}
