//! Withdraw all tokens from the Governance account to a receiver account.
//!
//! Only this program can call this instruction via a previous scheduled GMP
//! proposal, coming from the Axelar governance infrastructure.
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/InterchainGovernance.sol#L118).
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::state::GovernanceConfig;

use super::ensure_valid_governance_root_pda;

/// Withdraws all tokens from the Governance account to a receiver account.
/// Only the contract itself can call this instruction.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    amount: u64,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let _system_account = next_account_info(accounts_iter)?;
    let config_pda = next_account_info(accounts_iter)?;
    let receiver = next_account_info(accounts_iter)?;

    if !config_pda.is_signer {
        msg!("Only the contract itself can call this instruction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let config_data = config_pda.check_initialized_pda::<GovernanceConfig>(&crate::id())?;

    ensure_valid_governance_root_pda(config_data.bump, config_pda.key)?;

    match program_utils::transfer_lamports(config_pda, receiver, amount) {
        Ok(()) => {
            msg!(
                "{} lamports were transferred from {}",
                amount,
                config_pda.key
            );
            msg!("{} lamports were transferred to {}", amount, receiver.key);
            Ok(())
        }
        Err(err) => {
            msg!("Error transferring lamports: {:?}", err);
            Err(err)
        }
    }
}
