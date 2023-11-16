mod common;
mod errors;
pub mod instructions;

use anchor_lang::prelude::*;
use instructions::*;

declare_id!("C3iZqLs7omGNxbug6SbeKHAAiJYArNAkn9KxudeDSdpG");

#[program]
pub mod gateway {
    use super::*;

    pub fn call_contract(
        ctx: Context<CallContract>,
        destination_chain: String,
        destination_contract_address: String,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::call_contract(
            ctx,
            destination_chain,
            destination_contract_address,
            payload,
        )
        .unwrap();
        Ok(())
    }

    pub fn is_contract_call_approved(
        ctx: Context<IsContractCallApproved>,
        seeds_hash: [u8; 32],
    ) -> Result<bool> {
        Ok(instructions::is_contract_call_approved(ctx, seeds_hash).unwrap())
    }

    pub fn validate_contract_call(
        ctx: Context<ValidateContractCall>,
        seeds_hash: [u8; 32],
    ) -> Result<bool> {
        Ok(instructions::validate_contract_call(ctx, seeds_hash).unwrap())
    }

    pub fn execute(ctx: Context<Execute>, seeds_hash: [u8; 32]) -> Result<()> {
        instructions::execute(ctx, seeds_hash).unwrap();
        Ok(())
    }

    pub fn auth_module(ctx: Context<AuthModule>) -> Result<()> {
        instructions::auth_module(ctx).unwrap();
        Ok(())
    }

    pub fn is_command_executed(
        ctx: Context<IsCommandExecuted>,
        command_id: [u8; 32],
    ) -> Result<bool> {
        Ok(instructions::is_command_executed(ctx, command_id).unwrap())
    }
}
