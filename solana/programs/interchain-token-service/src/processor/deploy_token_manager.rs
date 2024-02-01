use interchain_token_transfer_gmp::DeployTokenManager;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;

impl Processor {
    /// Processes an instruction.
    pub fn deploy_token_manager(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo],
        _input: DeployTokenManager,
    ) -> ProgramResult {
        todo!()
    }
}
