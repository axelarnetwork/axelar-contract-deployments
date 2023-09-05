use super::*;

#[event]
pub struct ContractCallEvent {
    pub sender: Pubkey,
    pub destination_chain: String,
    pub destination_contract_address: String,
    pub payload_hash: [u8; 32],
    pub payload: Vec<u8>,
}

#[derive(Accounts)]
pub struct CallContract<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(signer)]
    pub sender: AccountInfo<'info>,
}

pub fn call_contract(
    ctx: Context<CallContract>,
    destination_chain: String,
    destination_contract_address: String,
    payload: Vec<u8>,
) -> Result<()> {
    emit!(ContractCallEvent {
        sender: ctx.accounts.sender.key(),
        destination_chain,
        destination_contract_address,
        payload_hash: keccak::hash(&payload).to_bytes(),
        payload,
    });

    Ok(())
}
