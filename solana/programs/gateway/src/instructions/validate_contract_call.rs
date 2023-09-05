use super::*;
use crate::{common::PREFIX_CONTRACT_CALL_APPROVED, state::State};

#[derive(Accounts)]
#[instruction(_command_id: [u8; 32], _source_chain: String, _source_address: String, _contract_address: String, _payload_hash: [u8; 32])]
pub struct ValidateContractCall<'info> {
    #[account(
        mut, 
        seeds = [PREFIX_CONTRACT_CALL_APPROVED.as_ref(), _command_id.as_ref(), _source_chain.as_ref(), _source_address.as_ref(), sender.key().as_ref(), _payload_hash.as_ref()],
        bump = state.bump
    )]
    pub state: Account<'info, State>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(signer)]
    pub sender: AccountInfo<'info>,
}

pub fn validate_contract_call(
    ctx: Context<ValidateContractCall>,
    _command_id: [u8; 32],
    _source_chain: String,
    _source_address: String,
    _payload_hash: [u8; 32],
) -> Result<bool> {
    let valid = ctx.accounts.state.value.clone();
    if valid {
        ctx.accounts.state.value = false;
    }
    Ok(valid)
}
