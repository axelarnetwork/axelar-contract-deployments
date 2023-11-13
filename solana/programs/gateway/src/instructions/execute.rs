use super::*;
use registry::cpi::accounts::Initialize;
use registry::program::Registry;
use registry::State;

#[event]
pub struct ExecutedEvent {
    pub seeds_hash: [u8; 32],
}

#[derive(Accounts)]
#[instruction(input: Vec<u8>)]
pub struct Execute<'info> {
    // TODO: add formula to generate PDA
    #[account(mut)]
    pub state: Account<'info, State>,

    /// CHECK: This is not dangerous because we don't read or write from this account; lie
    #[account(mut)]
    pub authority: Signer<'info>,
    ///
    pub registry_program: Program<'info, Registry>,
    pub system_program: Program<'info, System>,
}

// it should remain single message (not batch) due to maximum transaction size 1.2KB
pub fn execute(ctx: Context<Execute>, seeds_hash: [u8; 32]) -> Result<()> {
    // TODO: data / proof / validate proof

    // TODO: solidity contract has tree with if statements to recognise type of message
    // here it must be implemented differently

    // account with 0 lamports are not initialized
    if ctx.accounts.state.to_account_info().lamports() == 0 {
        let state_account = &mut ctx.accounts.state;
        let _ = registry::cpi::initialize(
            CpiContext::new(
                ctx.accounts.registry_program.to_account_info(),
                Initialize {
                    state: state_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            seeds_hash,
            // TODO: add comment why its false
            true,
        )?;

        emit!(ExecutedEvent { seeds_hash });
    }

    Ok(())
}
