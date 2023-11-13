use super::*;

use registry::cpi::accounts::Get;
use registry::program::Registry;
use registry::State;

#[derive(Accounts)]
pub struct IsContractCallApproved<'info> {
    // client side PDA // registry program id + PREFIX_CONTRACT_CALL_APPROVED + command_id + source_chain /
    // + source_address + contract_address + payload_hash
    pub state: Account<'info, State>,
    pub registry_program: Program<'info, Registry>,
}

// to calculate seeds_hash use PREFIX_CONTRACT_CALL_APPROVED + command_id + source_chain /
// + source_address + contract_address + payload_hash
pub fn is_contract_call_approved(
    ctx: Context<IsContractCallApproved>,
    seeds_hash: [u8; 32],
) -> Result<bool> {
    let result = registry::cpi::get(
        CpiContext::new(
            ctx.accounts.registry_program.to_account_info(),
            Get {
                state: ctx.accounts.state.to_account_info(),
            },
        ),
        seeds_hash,
    )?;

    Ok(result.get())
}
