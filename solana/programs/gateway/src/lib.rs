mod common;
mod errors;
mod instructions;
mod state;

use anchor_lang::prelude::*;
use instructions::*;

declare_id!("5cLTE6W6juWSW8cDvDonh6NRXPHvJwoLr3Kt9NFh3Wdp");

#[program]
pub mod gateway {
    use super::*;

    pub fn call_contract(
        ctx: Context<CallContract>,
        destination_chain: String,
        destination_contract_address: String,
        payload: Vec<u8>,
    ) -> Result<()> {
        let result = instructions::call_contract(
            ctx,
            destination_chain,
            destination_contract_address,
            payload,
        )
        .unwrap();

        Ok(result)
    }

    pub fn is_contract_call_approved(
        ctx: Context<IsContractCallApproved>,
        command_id: [u8; 32],
        source_chain: String,
        source_address: String,
        contract_address: String,
        payload_hash: [u8; 32],
    ) -> Result<bool> {
        let result = instructions::is_contract_call_approved(
            ctx,
            command_id,
            source_chain,
            source_address,
            contract_address,
            payload_hash,
        )
        .unwrap();

        Ok(result)
    }

    pub fn validate_contract_call(
        ctx: Context<ValidateContractCall>,
        command_id: [u8; 32],
        source_chain: String,
        source_address: String,
        payload_hash: [u8; 32],
    ) -> Result<bool> {
        let result = instructions::validate_contract_call(
            ctx,
            command_id,
            source_chain,
            source_address,
            payload_hash,
        )
        .unwrap();

        Ok(result)
    }

    pub fn execute(ctx: Context<Execute>, input: Vec<u8>) -> Result<()> {
        let result = instructions::execute(ctx, input).unwrap();

        Ok(result)
    }

    pub fn auth_module(ctx: Context<AuthModule>) -> Result<()> {
        let result = instructions::auth_module(ctx).unwrap();

        Ok(result)
    }

    pub fn is_command_executed(
        ctx: Context<IsCommandExecuted>,
        command_id: [u8; 32],
    ) -> Result<bool> {
        let result = instructions::is_command_executed(ctx, command_id).unwrap();

        Ok(result)
    }
}
