//! Instruction module for the Axelar Memo program.

use axelar_executable_old::axelar_message_primitives::DataPayload;
use axelar_executable_old::AxelarCallableInstruction;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
pub use solana_program;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

/// Instructions supported by the Axelar Memo program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum AxelarMemoInstruction {
    /// Initialize the memo program by creating a counter PDA
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [s] payer
    /// 1. [] gateway root pda
    /// 2. [w] counter PDA
    /// 3. [] system program
    Initialize {
        /// The pda bump for the counter PDA
        counter_pda_bump: u8,
    },

    /// Process a Memo
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [w] counter PDA
    ProcessMemo {
        /// The memo to receive
        memo: String,
    },

    /// Send a memo to a contract deployed on a different chain.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] gateway program id
    SendToGateway {
        /// Memo to send to the gateway
        memo: String,
        /// Destination chain we want to communicate with
        destination_chain: String,
        /// Destination contract address on the destination chain
        destination_address: String,
    },
}

/// Creates a [`AxelarMemoInstruction::Initialize`] instruction.
pub fn initialize(
    payer: &Pubkey,
    gateway_root_pda: &Pubkey,
    counter_pda: &(Pubkey, u8),
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarCallableInstruction::Native(to_vec(
        &AxelarMemoInstruction::Initialize {
            counter_pda_bump: counter_pda.1,
        },
    )?))?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new(counter_pda.0, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Create a [`AxelarMemoInstruction::SendToGateway`] instruction which will be
/// used to send a memo to the Solana gateway (create a message that's supposed
/// to land on an external chain)
pub fn call_gateway_with_memo(
    gateway_root_pda: &Pubkey,
    sender: &Pubkey,
    memo: String,
    destination_chain: String,
    destination_address: String,
) -> Result<Instruction, ProgramError> {
    let instruction_data =
        AxelarCallableInstruction::Native(to_vec(&AxelarMemoInstruction::SendToGateway {
            memo,
            destination_chain,
            destination_address,
        })?);
    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::ID, false),
    ];
    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: to_vec(&instruction_data)?,
    })
}

/// Helper function to build a memo payload instruction
pub mod from_axelar_to_solana {
    use axelar_executable_old::axelar_message_primitives::EncodingScheme;

    use super::*;

    /// Build a memo payload instruction
    pub fn build_memo<'a>(
        memo: &'a [u8],
        // The counter PDA that is going to be used in the memo
        counter_pda: &Pubkey,
        // The pubkeys that are going to be used in the memo just for logging purposes
        pubkeys: &[&Pubkey],
        encoding_scheme: EncodingScheme,
    ) -> DataPayload<'a> {
        let mut accounts = [counter_pda]
            .iter()
            .chain(pubkeys.iter())
            .map(|&pubkey| AccountMeta::new_readonly(*pubkey, false))
            .collect::<Vec<_>>();
        accounts[0].is_writable = true; // set the counter PDA to writable
        DataPayload::new(memo, accounts.as_slice(), encoding_scheme)
    }
}

#[cfg(test)]
mod tests {
    use axelar_executable_old::axelar_message_primitives::EncodingScheme;

    use super::*;

    #[test]
    fn test_build_memo() {
        let signer_pubkey = Pubkey::new_unique();
        let counter_pda = Pubkey::new_unique();
        let memo = "🐆".as_bytes();
        let instruction = from_axelar_to_solana::build_memo(
            memo,
            &counter_pda,
            &[&signer_pubkey],
            EncodingScheme::Borsh,
        );
        let payload = instruction.encode().unwrap();
        let instruction_decoded = DataPayload::decode(&payload).unwrap();

        assert_eq!(instruction, instruction_decoded);
    }
}
