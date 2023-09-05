use super::*;
use crate::state::State;

#[derive(Accounts)]
#[instruction(key: [u8; 32])]
pub struct InitBool<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    // space: 8 discriminator + 1 value + 1 bump
    #[account(
        init_if_needed,
        payer = sender,
        space = 8 + 1 + 1,
        seeds = [key.as_ref()],
        bump
    )]
    pub state: Account<'info, State>,
    pub system_program: Program<'info, System>,
}

fn _init_bool(ctx: Context<InitBool>, key: [u8; 32], value: bool) -> Result<()> {
    let state = &mut ctx.accounts.state;
    state.value = value;
    state.bump = *ctx.bumps.get("state").unwrap(); // TODO!: handle err
    msg!("new pda for key: {:?} created with value: {:?}", key, value);

    Ok(())
}
