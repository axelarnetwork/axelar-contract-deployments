use super::*;
use crate::state::State;

#[derive(Accounts)]
#[instruction(key: [u8; 32])]
pub struct DeleteBool<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        mut,
        close = sender,
        seeds = [key.as_ref()],
        bump = state.bump
    )]
    pub state: Account<'info, State>,
    pub system_program: Program<'info, System>,
}

fn _delete_bool(_ctx: Context<DeleteBool>, key: [u8; 32]) -> Result<()> {
    msg!("closed account of key: {:?}", key);

    Ok(())
}
