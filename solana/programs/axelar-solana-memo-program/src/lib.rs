#![deny(missing_docs)]

//! Simple memo program example for the Axelar Gateway on Solana

mod entrypoint;
pub mod processor;
use axelar_executable::axelar_message_primitives::{AxelarCallableInstruction, DataPayload};
use borsh::{BorshDeserialize, BorshSerialize};
pub use solana_program;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("Ra5JP1PPSsRP8idQfAWEdSrNCtkN4WTHRRtyxvpKp8C");

/// Instructions supported by the InterchainTokenService program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum AxelarMemoInstruction {
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
        destination_chain: Vec<u8>,
        /// Destination contract address on the destination chain
        destination_address: Vec<u8>,
    },
}

/// Create a [`AxelarMemoInstruction::SendToGateway`] instruction which will be
/// used to send a memo to the Solana gateway (create a message that's supposed
/// to land on an extenral chain)
pub fn call_gateway_with_memo(
    gateway_root_pda: &Pubkey,
    sender: &Pubkey,
    memo: String,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let instruction_data =
        AxelarCallableInstruction::Native(AxelarMemoInstruction::SendToGateway {
            memo,
            destination_chain,
            destination_address,
        });
    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::ID, false),
    ];
    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: borsh::to_vec(&instruction_data)?,
    })
}

/// Helper function to build a memo payload instruction (to simulate )
pub mod from_axelar_to_solana {
    use super::*;

    /// Build a memo payload instruction
    pub fn build_memo<'a>(memo: &'a [u8], pubkeys: &[&Pubkey]) -> DataPayload<'a> {
        let accounts = pubkeys
            .iter()
            .map(|&pubkey| AccountMeta::new_readonly(*pubkey, false))
            .collect::<Vec<_>>();
        DataPayload::new(memo, accounts.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_memo() {
        let signer_pubkey = Pubkey::new_unique();
        let memo = "🐆".as_bytes();
        let instruction = from_axelar_to_solana::build_memo(memo, &[&signer_pubkey]);
        let payload = instruction.encode();
        let instruction_decoded = DataPayload::decode(&payload);

        assert_eq!(instruction, instruction_decoded);
    }
}
