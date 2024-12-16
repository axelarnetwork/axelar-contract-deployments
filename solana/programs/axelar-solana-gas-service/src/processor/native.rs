use program_utils::{transfer_lamports, BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;

use crate::event_utils::{parse_u64_le, read_array, read_string, EventParseError};
use crate::state::Config;
use crate::{assert_valid_config_pda, event_prefixes};

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_pay_native_for_contract_call(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    params: &[u8],
    gas_fee_amount: u64,
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let system_program = next_account_info(accounts)?;

    {
        config_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = config_pda.try_borrow_data()?;
        let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
        assert_valid_config_pda(config.bump, &config.salt, &config.authority, config_pda.key)?;
    }

    invoke(
        &system_instruction::transfer(sender.key, config_pda.key, gas_fee_amount),
        &[sender.clone(), config_pda.clone(), system_program.clone()],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::NATIVE_GAS_PAID_FOR_CONTRACT_CALL,
        &config_pda.key.to_bytes(),
        &destination_chain.into_bytes(),
        &destination_address.into_bytes(),
        &payload_hash,
        &refund_address.to_bytes(),
        params,
        &gas_fee_amount.to_le_bytes(),
    ]);

    Ok(())
}

pub(crate) fn add_native_gas(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    log_index: u64,
    gas_fee_amount: u64,
    refund_address: Pubkey,
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let system_program = next_account_info(accounts)?;

    {
        config_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = config_pda.try_borrow_data()?;
        let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
        assert_valid_config_pda(config.bump, &config.salt, &config.authority, config_pda.key)?;
    }

    invoke(
        &system_instruction::transfer(sender.key, config_pda.key, gas_fee_amount),
        &[sender.clone(), config_pda.clone(), system_program.clone()],
    )?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::NATIVE_GAS_ADDED,
        &config_pda.key.to_bytes(),
        &tx_hash,
        &log_index.to_le_bytes(),
        &refund_address.to_bytes(),
        &gas_fee_amount.to_le_bytes(),
    ]);

    Ok(())
}

pub(crate) fn collect_fees_native(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    amount: u64,
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let authority = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let receiver = next_account_info(accounts)?;

    {
        // Check: Valid Config PDA
        config_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = config_pda.try_borrow_data()?;
        let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
        assert_valid_config_pda(config.bump, &config.salt, &config.authority, config_pda.key)?;

        // Check: Authority mtaches
        if authority.key != &config.authority {
            return Err(ProgramError::InvalidAccountOwner);
        }
    }

    // Check: Authority is signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    transfer_lamports(config_pda, receiver, amount)?;

    Ok(())
}

pub(crate) fn refund_native(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    log_index: u64,
    fees: u64,
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let authority = next_account_info(accounts)?;
    let receiver = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;

    {
        // Check: Valid Config PDA
        config_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = config_pda.try_borrow_data()?;
        let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
        assert_valid_config_pda(config.bump, &config.salt, &config.authority, config_pda.key)?;

        // Check: Authority mtaches
        if authority.key != &config.authority {
            return Err(ProgramError::InvalidAccountOwner);
        }
    }

    // Check: Authority is signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    transfer_lamports(config_pda, receiver, fees)?;

    // Emit an event
    sol_log_data(&[
        event_prefixes::NATIVE_GAS_REFUNDED,
        &tx_hash,
        &config_pda.key.to_bytes(),
        &log_index.to_le_bytes(),
        &receiver.key.to_bytes(),
        &fees.to_le_bytes(),
    ]);

    Ok(())
}

/// Represents the event emitted when native gas is paid for a contract call.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasPaidForContractCallEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
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

impl NativeGasPaidForContractCallEvent {
    /// Construct a new event from byte slices
    ///
    /// # Errors
    /// - if the data could not be parsed into an event
    pub fn new<I: Iterator<Item = Vec<u8>>>(mut data: I) -> Result<Self, EventParseError> {
        let config_pda_data = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda"))?;
        let config_pda = Pubkey::new_from_array(read_array::<32>("config_pda", &config_pda_data)?);

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
pub struct NativeGasAddedEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// index of the log
    pub log_index: u64,
    /// The refund address
    pub refund_address: Pubkey,
    /// amount of SOL
    pub gas_fee_amount: u64,
}

impl NativeGasAddedEvent {
    /// Construct a new event from byte slices
    ///
    /// # Errors
    /// - if the data could not be parsed into an event
    pub fn new<I: Iterator<Item = Vec<u8>>>(mut data: I) -> Result<Self, EventParseError> {
        let config_pda_data = data
            .next()
            .ok_or(EventParseError::MissingData("config_pda"))?;
        let config_pda = Pubkey::new_from_array(read_array::<32>("config_pda", &config_pda_data)?);

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
            tx_hash,
            log_index,
            refund_address,
            gas_fee_amount,
        })
    }
}

/// Represents the event emitted when native gas is refunded.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasRefundedEvent {
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

impl NativeGasRefundedEvent {
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
            tx_hash,
            config_pda,
            log_index,
            receiver,
            fees,
        })
    }
}
