//! Instruction module for the Axelar Memo program.

use anchor_discriminators_macros::InstructionDiscriminator;
use axelar_solana_gateway::executable::AxelarMessagePayload;
use borsh::to_vec;
pub use solana_program;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

/// Instructions supported by the Axelar Memo program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, InstructionDiscriminator)]
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
    /// 0. [] Memo program id
    /// 1. [w] Memo counter PDA
    /// 2. [] Memo program CALL CONTRACT signing PDA
    /// 3. [] gateway root pda
    /// 4. [] gateway program id
    SendToGateway {
        /// Memo to send to the gateway
        memo: String,
        /// Destination chain we want to communicate with
        destination_chain: String,
        /// Destination contract address on the destination chain
        destination_address: String,
    },

    /// Send an interchain token transfer initiated by the memo program's PDA.
    /// The source token account (counter PDA's ATA) is automatically derived and verified.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [w] Memo counter PDA (the actual token sender)
    /// 1. [] ITS root PDA
    /// 2. [] Token Manager PDA
    /// 3. [w] Source token account (counter PDA's ATA) - verified against derivation
    /// 4. [w] Token Manager's ATA
    /// 5. [] Gateway root PDA
    /// 6. [] Gateway program ID
    /// 7. [] Gas Service root PDA
    /// 8. [] Gas Service program ID
    /// 9. [] Token mint
    /// 10. [] Token program
    /// 11. [] Call contract signing PDA
    /// 12. [] ITS program ID
    /// 13. [] System program
    SendInterchainTransfer {
        /// Token ID for the transfer
        token_id: [u8; 32],
        /// Destination chain
        destination_chain: String,
        /// Destination address
        destination_address: Vec<u8>,
        /// Amount to transfer
        amount: u64,
        /// Gas value for the transfer
        gas_value: u128,
    },

    /// Send an interchain token transfer with intentionally wrong seeds (for testing)
    /// This instruction is identical to SendInterchainTransfer but uses incorrect seeds
    /// to test the validation logic in the ITS processor
    SendInterchainTransferWithWrongSeeds {
        /// Token ID for the transfer
        token_id: [u8; 32],
        /// Destination chain
        destination_chain: String,
        /// Destination address
        destination_address: Vec<u8>,
        /// Amount to transfer
        amount: u64,
        /// Gas value for the transfer
        gas_value: u128,
    },

    /// Send an interchain token transfer with additional data to call a contract on the destination
    /// This uses CpiCallContractWithInterchainToken to send tokens along with arbitrary data
    CallContractWithInterchainToken {
        /// Token ID for the transfer
        token_id: [u8; 32],
        /// Destination chain
        destination_chain: String,
        /// Destination address
        destination_address: Vec<u8>,
        /// Amount to transfer
        amount: u64,
        /// Additional data to pass to the destination contract
        data: Vec<u8>,
        /// Gas value for the transfer
        gas_value: u128,
    },
}

/// Creates a [`AxelarMemoInstruction::Initialize`] instruction.
pub fn initialize(payer: &Pubkey, counter_pda: &(Pubkey, u8)) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarMemoInstruction::Initialize {
        counter_pda_bump: counter_pda.1,
    })?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
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
    memo_counter_pda: &Pubkey,
    memo: String,
    destination_chain: String,
    destination_address: String,
    gateway_program_id: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarMemoInstruction::SendToGateway {
        memo,
        destination_chain,
        destination_address,
    })?;
    let signing_pda = axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let accounts = vec![
        AccountMeta::new_readonly(crate::ID, false),
        AccountMeta::new(*memo_counter_pda, false),
        AccountMeta::new_readonly(signing_pda.0, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gateway_program_id, false),
    ];
    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates a [`AxelarMemoInstruction::SendInterchainTransfer`] instruction.
/// The source token account (counter PDA's ATA) is automatically derived and verified inside the processor.
#[allow(clippy::too_many_arguments)]
pub fn send_interchain_transfer(
    payer: &Pubkey,
    memo_counter_pda: &Pubkey,
    its_root_pda: &Pubkey,
    token_manager_pda: &Pubkey,
    token_manager_ata: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    token_mint: &Pubkey,
    token_program: &Pubkey,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    gas_value: u128,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarMemoInstruction::SendInterchainTransfer {
        token_id,
        destination_chain,
        destination_address,
        amount,
        gas_value,
    })?;

    // Derive the source ATA (counter PDA's token account)
    let source_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        memo_counter_pda,
        token_mint,
        token_program,
    );

    // Additional required accounts for proper ITS instruction
    let gateway_program = axelar_solana_gateway::id();
    let gas_service_program = axelar_solana_gas_service::id();
    let (call_contract_signing_pda, _) =
        axelar_solana_gateway::get_call_contract_signing_pda(axelar_solana_its::id());
    let its_program = axelar_solana_its::id();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*memo_counter_pda, false),
        AccountMeta::new_readonly(*its_root_pda, false),
        AccountMeta::new(*token_manager_pda, false),
        AccountMeta::new(source_ata, false),
        AccountMeta::new(*token_manager_ata, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new(*gas_service_root_pda, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new(*token_mint, false),
        AccountMeta::new_readonly(*token_program, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(its_program, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates a [`AxelarMemoInstruction::SendInterchainTransferWithWrongSeeds`] instruction.
/// This is identical to `send_interchain_transfer` but will use wrong seeds for testing validation
#[allow(clippy::too_many_arguments)]
pub fn send_interchain_transfer_with_wrong_seeds(
    payer: &Pubkey,
    memo_counter_pda: &Pubkey,
    its_root_pda: &Pubkey,
    token_manager_pda: &Pubkey,
    token_manager_ata: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    token_mint: &Pubkey,
    token_program: &Pubkey,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    gas_value: u128,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(
        &AxelarMemoInstruction::SendInterchainTransferWithWrongSeeds {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
        },
    )?;

    // Derive the source ATA (counter PDA's token account)
    let source_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        memo_counter_pda,
        token_mint,
        token_program,
    );

    // Additional required accounts for proper ITS instruction
    let gateway_program = axelar_solana_gateway::id();
    let gas_service_program = axelar_solana_gas_service::id();
    let (call_contract_signing_pda, _) =
        axelar_solana_gateway::get_call_contract_signing_pda(axelar_solana_its::id());
    let its_program = axelar_solana_its::id();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*memo_counter_pda, false),
        AccountMeta::new_readonly(*its_root_pda, false),
        AccountMeta::new(*token_manager_pda, false),
        AccountMeta::new(source_ata, false),
        AccountMeta::new(*token_manager_ata, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new(*gas_service_root_pda, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new(*token_mint, false),
        AccountMeta::new_readonly(*token_program, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(its_program, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates a [`AxelarMemoInstruction::CallContractWithInterchainToken`] instruction.
/// This sends tokens along with additional data to call a contract on the destination
#[allow(clippy::too_many_arguments)]
pub fn call_contract_with_interchain_token(
    payer: &Pubkey,
    memo_counter_pda: &Pubkey,
    its_root_pda: &Pubkey,
    token_manager_pda: &Pubkey,
    token_manager_ata: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    token_mint: &Pubkey,
    token_program: &Pubkey,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    data: Vec<u8>,
    gas_value: u128,
) -> Result<Instruction, ProgramError> {
    let instruction_data = to_vec(&AxelarMemoInstruction::CallContractWithInterchainToken {
        token_id,
        destination_chain,
        destination_address,
        amount,
        data,
        gas_value,
    })?;

    // Derive the source ATA (counter PDA's token account)
    let source_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        memo_counter_pda,
        token_mint,
        token_program,
    );

    // Additional required accounts for proper ITS instruction
    let gateway_program = axelar_solana_gateway::id();
    let gas_service_program = axelar_solana_gas_service::id();
    let (call_contract_signing_pda, _) =
        axelar_solana_gateway::get_call_contract_signing_pda(axelar_solana_its::id());
    let its_program = axelar_solana_its::id();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*memo_counter_pda, false),
        AccountMeta::new_readonly(*its_root_pda, false),
        AccountMeta::new(*token_manager_pda, false),
        AccountMeta::new(source_ata, false),
        AccountMeta::new(*token_manager_ata, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new(*gas_service_root_pda, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new(*token_mint, false),
        AccountMeta::new_readonly(*token_program, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(its_program, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: instruction_data,
    })
}

/// Helper function to build a memo payload instruction
pub mod from_axelar_to_solana {
    use axelar_solana_gateway::executable::EncodingScheme;

    use super::*;

    /// Build a memo payload instruction
    pub fn build_memo<'a>(
        memo: &'a [u8],
        // The counter PDA that is going to be used in the memo
        counter_pda: &Pubkey,
        // The pubkeys that are going to be used in the memo just for logging purposes
        pubkeys: &[&Pubkey],
        encoding_scheme: EncodingScheme,
    ) -> AxelarMessagePayload<'a> {
        let mut accounts = [counter_pda]
            .iter()
            .chain(pubkeys.iter())
            .map(|&pubkey| AccountMeta::new_readonly(*pubkey, false))
            .collect::<Vec<_>>();
        accounts[0].is_writable = true; // set the counter PDA to writable
        AxelarMessagePayload::new(memo, accounts.as_slice(), encoding_scheme)
    }
}

#[cfg(test)]
mod tests {
    use axelar_solana_gateway::executable::EncodingScheme;

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
        let instruction_decoded = AxelarMessagePayload::decode(&payload).unwrap();

        assert_eq!(instruction, instruction_decoded);
    }
}
