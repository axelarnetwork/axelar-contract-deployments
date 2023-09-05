use super::*;

pub(crate) fn initialize(
    ctx: Context<Initialize>,
    _seeds_hash: [u8; 32],
    value: bool,
) -> Result<()> {
    let state_account = &mut ctx.accounts.state;
    state_account.value = value;
    state_account.authority = *ctx.accounts.authority.key;

    Ok(())
}

#[derive(Accounts)]
#[instruction(_seeds_hash: [u8; 32])]
pub struct Initialize<'info> {
    #[account(
        init,
        // 8-byte descriminator (anchor), 1-byte for bool and 32-byte for authority Pubkey
        space = 8 + 1 + 32,
        payer = authority,
        seeds = [_seeds_hash.as_ref()],
        bump
    )]
    pub state: Account<'info, State>,

    /// CHECK: This is not dangerous because we don't read or write from this account; lie
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn set(ctx: Context<Set>, _seeds_hash: [u8; 32], value: bool) -> Result<()> {
    let state_account = &mut ctx.accounts.state;
    state_account.value = value;

    Ok(())
}

#[derive(Accounts)]
#[instruction(_seeds_hash: [u8; 32])]
pub struct Set<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [_seeds_hash.as_ref()],
        bump
    )]
    pub state: Account<'info, State>,

    /// CHECK: This is not dangerous because we don't read or write from this account; lie
    #[account(mut)]
    pub authority: Signer<'info>,
}

pub(crate) fn get(ctx: Context<Get>, _seeds_hash: [u8; 32]) -> Result<bool> {
    Ok(ctx.accounts.state.value)
}

#[derive(Accounts)]
#[instruction(_seeds_hash: [u8; 32])]
pub struct Get<'info> {
    #[account(
        seeds = [_seeds_hash.as_ref()],
        bump
    )]
    pub state: Account<'info, State>,
}

pub(crate) fn delete(_ctx: Context<Delete>, _seeds_hash: [u8; 32]) -> Result<()> {
    Ok(())
}

#[derive(Accounts)]
#[instruction(_seeds_hash: [u8; 32])]
pub struct Delete<'info> {
    #[account(
        mut,
        close = authority,
        has_one = authority,
    )]
    pub state: Account<'info, State>,

    #[account(mut)]
    pub authority: Signer<'info>,
}
