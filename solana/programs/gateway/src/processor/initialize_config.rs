use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::axelar_auth_weighted::AxelarAuthWeighted;
use crate::error::GatewayError;
use crate::instructions::InitializeConfig;
use crate::state::GatewayConfig;
use crate::{assert_valid_gateway_root_pda, get_gateway_root_config_internal};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_config(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        init_config: InitializeConfig,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }
        let auth_weighted = AxelarAuthWeighted::new(
            init_config
                .initial_signer_sets
                .iter()
                .map(|x| x.parse().expect("invalid archived data")),
            init_config.previous_signers_retention,
            init_config.minimum_rotation_delay,
        );
        let (_, bump) = get_gateway_root_config_internal(program_id);
        let config = GatewayConfig::new(
            bump,
            auth_weighted,
            init_config.operator,
            init_config.domain_separator,
        );

        // Check: Gateway Config account uses the canonical bump.
        assert_valid_gateway_root_pda(config.bump, gateway_root_pda.key);
        // Check: Gateway Config account is not initialized.
        gateway_root_pda.check_uninitialized_pda()?;

        let bump = config.bump;
        program_utils::init_pda(
            payer,
            gateway_root_pda,
            program_id,
            system_account,
            config,
            &[b"gateway", &[bump]],
        )
    }
}
