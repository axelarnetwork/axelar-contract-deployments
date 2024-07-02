use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::commands::Command;
use crate::error::GatewayError;
use crate::state::{GatewayApprovedCommand, GatewayConfig};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_command(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        command: impl Command,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let approved_command_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }
        // Check: Gateway Root PDA Account is initialized
        let _gateway_root_pda =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        let (_canonical_pda, bump, seeds) =
            GatewayApprovedCommand::pda(gateway_root_pda.key, &command);
        let comamnd = GatewayApprovedCommand::pending(bump, &command);

        // Check: Approved Message account is not initialized.
        approved_command_pda.check_uninitialized_pda()?;
        // Check: Approved message account uses the canonical bump.
        comamnd.assert_valid_pda(&seeds, approved_command_pda.key);

        program_utils::init_pda(
            payer,
            approved_command_pda,
            program_id,
            system_account,
            comamnd,
            &[seeds.as_ref(), &[bump]],
        )
    }
}
