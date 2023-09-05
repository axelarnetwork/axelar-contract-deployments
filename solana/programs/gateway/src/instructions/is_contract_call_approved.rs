use super::*;
use crate::{common::PREFIX_CONTRACT_CALL_APPROVED, state::State};

#[derive(Accounts)]
#[instruction(_command_id: [u8; 32], _source_chain: String, _source_address: String, _contract_address: String, _payload_hash: [u8; 32])]
pub struct IsContractCallApproved<'info> {
    #[account(
        seeds = [PREFIX_CONTRACT_CALL_APPROVED.as_ref(), _command_id.as_ref(), _source_chain.as_ref(), _source_address.as_ref(), _contract_address.as_ref(), _payload_hash.as_ref()],
        bump = state.bump
    )]
    pub state: Account<'info, State>,
}

pub fn is_contract_call_approved(
    ctx: Context<IsContractCallApproved>,
    _command_id: [u8; 32],
    _source_chain: String,
    _source_address: String,
    _contract_address: String,
    _payload_hash: [u8; 32],
) -> Result<bool> {
    Ok(ctx.accounts.state.value)
}
