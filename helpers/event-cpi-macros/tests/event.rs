#![cfg(test)]

use anchor_discriminators::Discriminator;
use event_cpi::CpiEvent;
use event_cpi_macros::{emit_cpi, event, event_cpi_accounts};
use solana_program::pubkey::Pubkey;
use solana_sdk::{account_info::AccountInfo, clock::Epoch, program_error::ProgramError};

solana_program::declare_id!("gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp");

/// Represents the event emitted when native gas is paid for a contract call.
#[event]
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

#[test]
fn test_discriminator() {
    let event = NativeGasPaidForContractCallEvent {
        config_pda: Pubkey::new_unique(),
        destination_chain: "chain".to_owned(),
        destination_address: "address".to_owned(),
        payload_hash: [0u8; 32],
        refund_address: Pubkey::new_unique(),
        params: vec![1, 2, 3],
        gas_fee_amount: 100,
    };

    let data = event.data();
    #[allow(clippy::indexing_slicing)]
    let data = &data[..8];
    assert_eq!(data, NativeGasPaidForContractCallEvent::DISCRIMINATOR);
}

#[test]
fn test_emit_cpi() -> Result<(), ProgramError> {
    let event = NativeGasPaidForContractCallEvent {
        config_pda: Pubkey::new_unique(),
        destination_chain: "chain".to_owned(),
        destination_address: "address".to_owned(),
        payload_hash: [0u8; 32],
        refund_address: Pubkey::new_unique(),
        params: vec![1, 2, 3],
        gas_fee_amount: 100,
    };

    // Create test accounts
    let (event_authority_key, _bump) =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::ID);
    let program_key = crate::ID;

    // Create AccountInfo structs for testing
    let mut event_authority_lamports = 1_000_000;
    let mut event_authority_data = vec![0u8; 32];
    let event_authority_account = AccountInfo::new(
        &event_authority_key,
        false, // is_signer
        false, // is_writable
        &mut event_authority_lamports,
        &mut event_authority_data,
        &program_key,
        false, // is_executable
        Epoch::default(),
    );

    let mut program_lamports = 1_000_000;
    let mut program_data = vec![0u8; 32];
    let program_account = AccountInfo::new(
        &program_key,
        false, // is_signer
        false, // is_writable
        &mut program_lamports,
        &mut program_data,
        &program_key,
        true, // is_executable
        Epoch::default(),
    );

    let accounts: Vec<AccountInfo> = vec![event_authority_account, program_account];
    let accounts_slice: &[AccountInfo] = &accounts;
    let accounts = &mut accounts_slice.iter();

    event_cpi_accounts!();

    emit_cpi!(event);

    Ok(())
}
