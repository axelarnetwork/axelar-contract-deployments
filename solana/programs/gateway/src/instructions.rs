//! Instruction types

use axelar_rkyv_encoding::types::Message;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::commands::{MessageWrapper, OwnedCommand};
use crate::state::{GatewayApprovedCommand, GatewayConfig, GatewayExecuteData};

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum GatewayInstruction {
    /// Processes incoming batch of ApproveMessage commands from Axelar
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Gateway ExecuteData PDA account
    /// 2..N [WRITE] Gateway ApprovedCommand PDA accounts. All commands needs to
    ///         be `ApproveMessages`.
    ApproveMessages,

    /// Rotate signers for the Gateway Root Config PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE] Gateway Root Config PDA account
    /// 1. [] Gateway ExecuteData PDA account
    /// 2. [WRITE] Gateway ApprovedCommand PDA account. The command needs to be
    ///    `RotateSigners`.
    /// 3. Opional: [SIGNER] `Operator` that's stored in the gateway confi PDA.
    RotateSigners,

    /// Represents the `CallContract` Axelar event.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Sender (origin) of the message)
    /// 1. [] Gateway Root Config PDA account
    CallContract {
        /// The name of the target blockchain.
        destination_chain: Vec<u8>,
        /// The address of the target contract in the destination blockchain.
        destination_contract_address: Vec<u8>,
        /// Contract call data.
        payload: Vec<u8>,
    },

    /// Initializes the Gateway configuration PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Gateway Root Config PDA account
    /// 2. [] System Program account
    InitializeConfig {
        /// Initial state of the root PDA `Config`.
        config: GatewayConfig,
    }, // XXX

    /// Initializes an Execute Data PDA account.
    /// The Execute Data is a batch of commands that will be executed by the
    /// Execute instruction (separate step). The `execute_data` will be
    /// decoded on-chain to verify the data is correct and generate the proper
    /// hash, and store it in the Execute Data PDA account.
    ///
    /// It's expected that for each command in the batch, there is a
    /// corresponding `GatewayApprovedCommand` account. The sequence of
    /// which is initialized first is not important.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Execute Data PDA account
    /// 2. [] System Program account
    InitializeExecuteData {
        /// The execute data that will be decoded.
        /// We decode it on-chain so we can verify the data is correct and
        /// generate the proper hash.
        execute_data: Vec<u8>,
    }, // XXX

    /// Initializes a pending command.
    /// This instruction is used to initialize a command that will trackt he
    /// execution state of a command contained in a batch.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Gateway ApprovedCommand PDA account
    /// 2. [] Gateway Root Config PDA account
    /// 3. [] System Program account
    InitializePendingCommand(OwnedCommand), // XXX

    /// Validates message.
    /// It is the responsibility of the destination program (contract) that
    /// receives a message from Axelar to validate that the message has been
    /// approved by the Gateway.
    ///
    /// Once the message has been validated, the command will no longer be valid
    /// for future calls.
    ///
    /// Accounts expected by this instruction:
    /// 1. [WRITE] Approved Message PDA account
    /// 2. [] Gateway Root Config PDA account
    /// 3. [SIGNER] PDA signer account (caller). Dervied from the destination
    ///    program id.
    ValidateMessage(MessageWrapper), // XXX

    /// Transfers operatorship of the Gateway Root Config PDA account.
    ///
    /// Only the current operator OR Gateway program owner can transfer
    /// operatorship to a new operator.
    ///
    /// Accounts expected by this instruction:
    /// 1. [WRITE] Config PDA account
    /// 2. [SIGNER] Current operator OR the upgrade authority of the Gateway
    ///    programdata account
    /// 3. [] Gateway programdata account (owned by `bpf_loader_upgradeable`)
    /// 4. [] New operator
    TransferOperatorship,
}

/// Creates a [`GatewayInstruction::ApproveMessages`] instruction.
pub fn approve_messages(
    // todo: we don't need to expose the program id
    program_id: Pubkey,
    execute_data_account: Pubkey,
    gateway_root_pda: Pubkey,
    command_accounts: &[Pubkey],
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(program_id)?;
    let data = to_vec(&GatewayInstruction::ApproveMessages)?;

    let mut accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new(execute_data_account, false),
    ];

    // Message accounts needs to be writable so we can set them as processed.
    accounts.extend(
        command_accounts
            .iter()
            .map(|key| AccountMeta::new(*key, false)),
    );

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::RotateSigners`] instruction.
pub fn rotate_signers(
    // todo: we don't need to expose the program id here
    program_id: Pubkey,
    execute_data_account: Pubkey,
    gateway_root_pda: Pubkey,
    command_account: Pubkey,
    operator: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(program_id)?;
    let data = to_vec(&GatewayInstruction::RotateSigners)?;

    let mut accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new(execute_data_account, false),
        AccountMeta::new(command_account, false),
    ];

    if let Some(operator) = operator {
        accounts.push(AccountMeta::new(operator, true));
    }

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Helper to create an instruction with the given ExecuteData and accounts.
#[deprecated = "Use `rotate_signers` or `approve_messages` instead"]
pub fn handle_execute_data(
    gateway_root_pda: Pubkey,
    execute_data_account: Pubkey,
    command_accounts: &[Pubkey],
    // todo: we don't need to expose the program id here
    program_id: Pubkey,
    data: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let mut accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new(execute_data_account, false),
    ];

    // Message accounts needs to be writable so we can set them as processed.
    accounts.extend(
        command_accounts
            .iter()
            .map(|key| AccountMeta::new(*key, false)),
    );

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Creates a [`CallContract`] instruction.
pub fn call_contract(
    gateway_root_pda: Pubkey,
    sender: Pubkey,
    destination_chain: Vec<u8>,
    destination_contract_address: Vec<u8>,
    payload: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::CallContract {
        destination_chain,
        destination_contract_address,
        payload,
    })?;

    let accounts = vec![
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializePendingCommand`] instruction.
pub fn initialize_pending_command(
    gateway_root_pda: &Pubkey,
    payer: &Pubkey,
    command: OwnedCommand,
) -> Result<Instruction, ProgramError> {
    let (approved_message_pda, _bump, _seed) =
        GatewayApprovedCommand::pda(gateway_root_pda, &command);

    let data = to_vec(&GatewayInstruction::InitializePendingCommand(command))?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(approved_message_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeExecuteData`] instruction.
pub fn initialize_execute_data(
    payer: Pubkey,
    gateway_root_pda: Pubkey,
    // The encoded data that will be decoded on-chain.
    raw_execute_data: &[u8],
) -> Result<(Instruction, GatewayExecuteData<'_>), ProgramError> {
    // We decode the data off-chain so we can find its PDA.
    let decoded_execute_data = GatewayExecuteData::new(raw_execute_data, &gateway_root_pda)?;
    let (execute_data_pda, _, _) = decoded_execute_data.pda(&gateway_root_pda);
    // We store the raw data so we can verify it on-chain.
    let data = to_vec(&GatewayInstruction::InitializeExecuteData {
        // TODO: Try to avoid allocating the data twice here, as both `borsh::to_vec`
        // and `std::slice::to_vec` copy the execute_data bytes into new vectors.
        execute_data: raw_execute_data.to_vec(),
    })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(execute_data_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok((
        Instruction {
            program_id: crate::id(),
            accounts,
            data,
        },
        decoded_execute_data,
    ))
}

/// Creates a [`GatewayInstruction::InitializeConfig`] instruction.
pub fn initialize_config(
    payer: Pubkey,
    config: GatewayConfig,
    gateway_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::InitializeConfig { config })?;
    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(gateway_config_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstructon::ValidateMessage`] instruction.
pub fn validate_message(
    approved_message_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
    caller: &Pubkey,
    message: Message,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*approved_message_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*caller, true),
    ];

    let message_wrapper = message.try_into()?;
    let data = borsh::to_vec(&GatewayInstruction::ValidateMessage(message_wrapper))?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::TransferOperatorship`] instruction.
pub fn transfer_operatorship(
    gateway_root_pda: Pubkey,
    current_operator_or_gateway_program_owner: Pubkey,
    new_operator: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (programdata_pubkey, _) =
        Pubkey::try_find_program_address(&[crate::id().as_ref()], &bpf_loader_upgradeable::id())
            .ok_or(ProgramError::IncorrectProgramId)?;
    let accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new_readonly(current_operator_or_gateway_program_owner, true),
        AccountMeta::new_readonly(programdata_pubkey, false),
        AccountMeta::new_readonly(new_operator, false),
    ];

    let data = borsh::to_vec(&GatewayInstruction::TransferOperatorship)?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

#[cfg(test)]
pub mod tests {

    use borsh::from_slice;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;

    use super::*;

    #[test]
    fn round_trip_queue() {
        let original = GatewayInstruction::ApproveMessages {};
        let serialized = to_vec(&original).unwrap();
        let deserialized = from_slice::<GatewayInstruction>(&serialized).unwrap();
        assert_eq!(deserialized, original);
    }

    #[test]
    fn round_trip_queue_function() {
        let execute_data_account = Keypair::new().pubkey();
        let _payer = Keypair::new().pubkey();
        let (gateway_root_pda, _) = GatewayConfig::pda();
        let approved_message_accounts = vec![Keypair::new().pubkey()];
        let instruction = approve_messages(
            crate::id(),
            execute_data_account,
            gateway_root_pda,
            &approved_message_accounts,
        )
        .expect("valid instruction construction");
        let deserialized = from_slice(&instruction.data).expect("deserialized valid instruction");
        assert!(matches!(
            deserialized,
            GatewayInstruction::ApproveMessages {}
        ));
    }

    #[test]
    fn round_trip_call_contract() {
        let destination_chain = "ethereum".as_bytes().to_vec();
        let destination_contract_address =
            hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862").unwrap();
        let payload = vec![5; 100];

        let instruction = GatewayInstruction::CallContract {
            destination_chain: destination_chain.to_owned(),
            destination_contract_address,
            payload,
        };

        let serialized = to_vec(&instruction).expect("call contract to be serialized");
        let deserialized = from_slice(&serialized).expect("call contract to be deserialized");

        assert_eq!(instruction, deserialized);
    }

    #[test]
    fn round_trip_call_contract_function() {
        let sender = Keypair::new().pubkey();
        let destination_chain = "ethereum".as_bytes().to_vec();
        let destination_contract_address =
            hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862").unwrap();
        let payload = vec![5; 100];

        let instruction = call_contract(
            crate::id(),
            sender,
            destination_chain.clone(),
            destination_contract_address.clone(),
            payload.clone(),
        )
        .expect("valid instruction construction");

        let deserialized = from_slice(&instruction.data).expect("deserialize valid instruction");

        match deserialized {
            GatewayInstruction::CallContract {
                destination_chain: deserialized_destination_chain,
                destination_contract_address: deserialized_destination_contract_address,
                payload: deserialized_payload,
            } => {
                assert_eq!(destination_chain, deserialized_destination_chain);
                assert_eq!(
                    destination_contract_address,
                    deserialized_destination_contract_address
                );
                assert_eq!(payload.as_slice(), deserialized_payload.as_slice());
            }
            _ => panic!("Wrong instruction"),
        };
    }
}
