use super::*;

use registry::cpi::accounts::{Get, Set};
use registry::program::Registry;
use registry::State;

#[derive(Accounts)]
pub struct ValidateContractCall<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    // client side PDA // registry program id + PREFIX_CONTRACT_CALL_APPROVED + command_id + source_chain /
    // + source_address + payload_hash
    #[account(mut)]
    pub state: Account<'info, State>,
    pub registry_program: Program<'info, Registry>,
}

// seeds_hash // PREFIX_CONTRACT_CALL_APPROVED + command_id + source_chain /
// + source_address + payload_hash
pub fn validate_contract_call(
    ctx: Context<ValidateContractCall>,
    seeds_hash: [u8; 32],
) -> Result<bool> {
    let state_account = &ctx.accounts.state;
    let result = registry::cpi::get(
        CpiContext::new(
            ctx.accounts.registry_program.to_account_info(),
            Get {
                state: state_account.to_account_info(),
            },
        ),
        seeds_hash,
    )?;

    let valid = result.get();
    if valid {
        let state_account = &mut ctx.accounts.state;
        let _ = registry::cpi::set(
            CpiContext::new(
                ctx.accounts.registry_program.to_account_info(),
                Set {
                    state: state_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            seeds_hash,
            // TODO: add comment why its false
            false,
        )?;
    }

    Ok(valid)
}
