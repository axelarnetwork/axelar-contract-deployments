use super::*;
use crate::common::execute::{Data, Input};
use crate::common::PREFIX_COMMAND_EXECUTED;
use crate::errors::Error;
use crate::state::State;

#[event]
pub struct ExecuteEvent {
    pub command_id: [u8; 32],
}

#[derive(Accounts)]
#[instruction(input: Vec<u8>)]
pub struct Execute<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account; lie
    #[account(mut, signer)]
    pub payer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

// TODO! add check to make sure that caller isnt contract itself / Solidity External
pub fn execute(_ctx: Context<Execute>, input: Vec<u8>) -> Result<()> {
    let decoded_input = Input::decode(input);
    let _message_hash = keccak::hash(&decoded_input.data);
    let decoded_data = Data::decode(decoded_input.data);

    // returns true for current operators
    // bool allowOperatorshipTransfer = AUTH_MODULE.validateProof(messageHash, proof);
    //
    // DUMMY / TODO!: AuthModule
    let _allow_operatorship_transfer = true;

    // TODO!: if (chainId != block.chainid) revert InvalidChainId();

    let commands_len = decoded_data.command_ids.len();
    require!(
        commands_len != decoded_data.commands.len() || commands_len != decoded_data.params.len(),
        Error::InvalidCommands
    );

    for command_id in decoded_data.command_ids {
        // TODO!: if (isCommandExecuted(commandId)) continue; /* Ignore if duplicate commandId received */
        msg!("magic with auth / commands / TODO");

        //
        // TODO!: Prevent a re-entrancy from executing this command before it can be marked as successful.
        // _setCommandExecuted(commandId, true);

        // DUMMY
        let success = true;

        if success {
            emit!(ExecuteEvent { command_id })
        } else {
            todo!()
        }
    }

    Ok(())
}
