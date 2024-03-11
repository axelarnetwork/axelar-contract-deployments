use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::error::GatewayError;
use crate::state::GatewayConfig;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_config(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        config: GatewayConfig,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Gateway Config account uses the canonical bump.
        let (canonical_pda, canonical_bump) = crate::get_gateway_root_config_pda();
        if *gateway_root_pda.key != canonical_pda {
            return Err(GatewayError::InvalidConfigAccount.into());
        }

        super::init_pda(payer, gateway_root_pda, &[&[canonical_bump]], &config)
    }
}
