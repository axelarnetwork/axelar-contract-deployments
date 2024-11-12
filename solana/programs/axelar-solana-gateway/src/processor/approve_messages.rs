use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;

impl Processor {
    /// Approves an array of messages, signed by the Axelar signers.
    /// reference implementation: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/2eaf5199ee8ccc5eb1d8353c0dd7592feff0eb5c/contracts/gateway/AxelarAmplifierGateway.sol#L78-L84
    pub fn process_approve_messages(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo<'_>],
    ) -> ProgramResult {
        Ok(())
    }
}
