#![cfg(test)]

/// This test imitates the account parsing done in ITS and governance programs
use event_cpi::Discriminator;
use event_cpi_macros::{emit_cpi, event, event_cpi_accounts};
use program_utils::account_array_structs;
use solana_program::pubkey::Pubkey;
use solana_sdk::{account_info::AccountInfo, clock::Epoch, program_error::ProgramError};

solana_program::declare_id!("gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp");

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    ExecuteOperatorProposalInfo,
    // Struct whose attributes are of type `AccountMeta`
    ExecuteOperatorProposalMeta,
    // Attributes
    system_account
}

/// Represents the event emitted when native gas is paid for a contract call.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperatorProposalExecuted {
    /// The hash of the proposal in which the PDA derivation was based.
    /// The hash is crafted from the target address, call data and native
    /// value.
    hash: [u8; 32],
    /// The target address represented as a 32-byte array. It represents the
    /// [`solana_program::pubkey::Pubkey`].
    target_address: [u8; 32],
    /// The call data required to execute the target program.
    /// See [`crate::proposal::ExecuteProposalCallData`].
    call_data: Vec<u8>,
    /// This field represents how many native tokens (lamports) are being
    /// sent to the target program. It's a little-endian U256 value.
    native_value: [u8; 32],
}

#[test]
fn test_emit_cpi_account_parsing() -> Result<(), ProgramError> {
    let event = OperatorProposalExecuted {
        hash: [0u8; 32],
        target_address: [0u8; 32],
        call_data: vec![],
        native_value: [0u8; 32],
    };

    // Create test accounts
    let (event_authority_key, _bump) =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::ID);
    let program_key = crate::ID;

    // Create AccountInfo structs for testing
    // The actual accounts are not important

    let mut system_account_lamports = 1_000_000;
    let mut system_account_data = vec![0u8; 32];
    let system_account = AccountInfo::new(
        &solana_program::system_program::ID,
        false, // is_signer
        false, // is_writable
        &mut system_account_lamports,
        &mut system_account_data,
        &solana_program::system_program::ID,
        true, // is_executable
        Epoch::default(),
    );

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

    // Imitate the ITS and governance style of accounts parsing

    let accounts: Vec<AccountInfo> = vec![system_account, event_authority_account, program_account];
    let accounts_slice: &[AccountInfo] = &accounts;
    let accounts = &mut accounts_slice.iter();

    let ExecuteOperatorProposalInfo {
        system_account: _system_account,
    } = ExecuteOperatorProposalInfo::from_account_iter(accounts)?;

    event_cpi_accounts!();

    emit_cpi!(event);

    Ok(())
}
