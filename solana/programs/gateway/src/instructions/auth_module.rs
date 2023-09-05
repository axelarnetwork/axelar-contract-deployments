use super::*;

#[derive(Accounts)]
pub struct AuthModule<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(signer)]
    pub sender: AccountInfo<'info>,
}

pub fn auth_module(_ctx: Context<AuthModule>) -> Result<()> {
    Ok(())
}
