use super::*;
use crate::{common::PREFIX_COMMAND_EXECUTED, state::State};

#[derive(Accounts)]
#[instruction(_command_id: [u8; 32])]
pub struct IsCommandExecuted<'info> {
    #[account(
        seeds = [PREFIX_COMMAND_EXECUTED.as_ref(), _command_id.as_ref()],
        bump = state.bump
    )]
    pub state: Account<'info, State>,
}

pub fn is_command_executed(ctx: Context<IsCommandExecuted>, _command_id: [u8; 32]) -> Result<bool> {
    Ok(ctx.accounts.state.value)
}
