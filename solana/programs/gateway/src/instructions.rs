//! Instruction types

use axelar_message_primitives::{AxelarMessageParams, DestinationProgramId};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::accounts::transfer_operatorship::TransferOperatorshipAccount;
use crate::accounts::{GatewayApprovedMessage, GatewayConfig, GatewayExecuteData};
use crate::get_gateway_root_config_pda;
use crate::types::address::Address;
use crate::types::u256::U256;

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum GatewayInstruction {
    /// Represents the `CallContract` Axelar event.
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Config PDA account
    /// 1. [WRITE] Execute Data PDA account
    /// N. [WRITE] Approved Message PDA accounts
    Execute,

    /// Represents the `CallContract` Axelar event.
    ///
    /// No accounts are expected by this instruction.
    CallContract {
        /// The name of the target blockchain.
        destination_chain: String,
        /// The address of the target contract in the destination blockchain.
        destination_contract_address: Vec<u8>,
        /// Contract call data.
        payload: Vec<u8>,
    },

    /// Initializes the Gateway configuration PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Gateway Config PDA account
    /// 2. [] System Program account
    InitializeConfig {
        /// Initial state of the root PDA `Config`.
        config: GatewayConfig,
    },

    /// Recieves parameters over account.
    ///
    /// Is meant to be used as part of key rotation process.
    TransferOperatorship,

    /// Initializes an Execute Data PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Execute Data PDA account
    /// 2. [] System Program account
    InitializeExecuteData {
        /// The execute data that will be decoded.
        execute_data: GatewayExecuteData,
    },

    /// Initializes an Approved Message PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Approved Message PDA account
    /// 2. [] System Program account
    // TODO: could we just pass `connection_router::Message` directly here? It
    //       would eliminate the need for "leaving conversions to the caller's
    //       discretion"
    InitializeMessage {
        /// The Axelar Message CCID, truncated to 32 bytes during proof
        /// generation.
        command_id: [u8; 32],
        /// The source chain denomination
        source_chain: String,
        /// The source address, expressed as raw bytes, leaving conversions to
        /// the caller's discretion.
        source_address: Vec<u8>,
        /// The Axelar Message payload hash.
        payload_hash: [u8; 32],
        /// This is parsed `axelar_message.destination_address` (which
        /// must be a valid program id)
        destination_program: DestinationProgramId,
    },
    /// Initializes the account to hold a new operator set.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] New operator set PDA account
    /// 2. [] System Program account
    InitializeTransferOperatorship {
        /// List of operator addresses and their weights.
        operators_and_weights: Vec<(Address, U256)>,
        /// Threshold for this operator set
        threshold: U256,
    },
    /// Validates a contract call.
    ///
    /// Accounts expected by this instruction:
    /// 1. [WRITE] Approved Message PDA account
    /// 2. [] Gateway Config PDA account
    /// 3. [SIGNER] PDA signer account (caller)
    ValidateContractCall {
        /// The Axelar Message CCID, truncated to 32 bytes during proof
        /// generation.
        command_id: [u8; 32],
        /// The source chain denomination
        source_chain: String,
        /// The source address, expressed as raw bytes, leaving conversions to
        /// the caller's discretion.
        source_address: Vec<u8>,
        /// The Axelar Message payload hash.
        payload_hash: [u8; 32],
        /// This is parsed `axelar_message.destination_address` (which
        /// must be a valid program id)
        destination_program: DestinationProgramId,
    },
}

/// Creates a [`GatewayInstruction::Execute`] instruction.
pub fn execute(
    program_id: Pubkey,
    execute_data_account: Pubkey,
    gateway_root_pda: Pubkey,
    message_accounts: &[Pubkey],
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(program_id)?;

    if message_accounts.is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }

    let data = to_vec(&GatewayInstruction::Execute {})?;

    let mut accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(execute_data_account, false),
    ];

    // Message accounts needs to be writable so we can set them as processed.
    accounts.extend(
        message_accounts
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
    program_id: Pubkey,
    sender: Pubkey,
    destination_chain: &str,
    destination_contract_address: &[u8],
    payload: &[u8],
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(program_id)?;

    let data = to_vec(&GatewayInstruction::CallContract {
        destination_chain: destination_chain.to_owned(),
        destination_contract_address: destination_contract_address.to_vec(),
        payload: payload.to_vec(),
    })?;

    let accounts = vec![AccountMeta::new_readonly(sender, true)];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeMessage`] instruction.
pub fn initialize_message<'a>(
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    message: impl Into<AxelarMessageParams<'a>>,
) -> Result<Instruction, ProgramError> {
    let message = message.into();
    let (approved_message_pda, _bump, _seed) =
        GatewayApprovedMessage::pda(&gateway_root_pda, &message);

    let data = to_vec(&GatewayInstruction::InitializeMessage {
        command_id: *message.command_id.0.as_ref(),
        source_chain: message.source_chain.0.to_string(),
        source_address: message.source_address.0.into(),
        payload_hash: *message.payload_hash.0,
        destination_program: message.destination_program,
    })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(approved_message_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
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
    execute_data: GatewayExecuteData,
) -> Result<Instruction, ProgramError> {
    let (execute_data_pda, _, _) = execute_data.pda();
    let data = to_vec(&GatewayInstruction::InitializeExecuteData { execute_data })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(execute_data_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeConfig`] instruction.
pub fn initialize_config(
    payer: Pubkey,
    config: GatewayConfig,
) -> Result<Instruction, ProgramError> {
    let (gateway_config_pda, _bump) = get_gateway_root_config_pda();
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

/// Creates a [`GatewayInstructon::TransferOperatorship`] instruction
pub fn transfer_operatorship(
    payer: &Pubkey,
    new_operators: &Pubkey,
    state: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*new_operators, false),
        AccountMeta::new(*state, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let data = borsh::to_vec(&GatewayInstruction::TransferOperatorship {})?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstructon::InitializeTransferOperatorship`] instruction.
pub fn initialize_trasfer_operatorship(
    payer: &Pubkey,
    operators_and_weights: Vec<(Address, U256)>,
    threshold: U256,
) -> Result<Instruction, ProgramError> {
    let transfer_operatorship_account =
        TransferOperatorshipAccount::new(operators_and_weights.clone(), threshold);

    let (pda, _bump) = transfer_operatorship_account.pda();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let data = borsh::to_vec(&GatewayInstruction::InitializeTransferOperatorship {
        operators_and_weights,
        threshold,
    })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstructon::ValidateContractCall`] instruction.
pub fn validate_contract_call<'a>(
    approved_message_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
    caller: &Pubkey,
    message: impl Into<AxelarMessageParams<'a>>,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*approved_message_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*caller, true),
    ];

    let params = message.into();

    let data = borsh::to_vec(&GatewayInstruction::ValidateContractCall {
        destination_program: params.destination_program,
        command_id: *params.command_id.0.as_ref(),
        payload_hash: *params.payload_hash.0,
        source_address: params.source_address.0.into(),
        source_chain: params.source_chain.0.to_string(),
    })?;

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
    use test_fixtures::primitives::bytes;

    use super::*;

    #[test]
    fn round_trip_queue() {
        let original = GatewayInstruction::Execute {};
        let serialized = to_vec(&original).unwrap();
        let deserialized = from_slice::<GatewayInstruction>(&serialized).unwrap();
        assert_eq!(deserialized, original);
    }

    #[test]
    fn round_trip_queue_function() {
        let execute_data_account = Keypair::new().pubkey();
        let (gateway_root_pda, _) = GatewayConfig::pda();
        let approved_message_accounts = vec![Keypair::new().pubkey()];
        let instruction = execute(
            crate::id(),
            execute_data_account,
            gateway_root_pda,
            &approved_message_accounts,
        )
        .expect("valid instruction construction");
        let deserialized = from_slice(&instruction.data).expect("deserialized valid instruction");
        assert!(matches!(deserialized, GatewayInstruction::Execute {}));
    }

    #[test]
    fn round_trip_call_contract() {
        let destination_chain = "ethereum";
        let destination_contract_address =
            hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862").unwrap();
        let payload = bytes(100);

        let instruction = GatewayInstruction::CallContract {
            destination_chain: destination_chain.to_owned(),
            destination_contract_address,
            payload: payload.to_vec(),
        };

        let serialized = to_vec(&instruction).expect("call contract to be serialized");
        let deserialized = from_slice(&serialized).expect("call contract to be deserialized");

        assert_eq!(instruction, deserialized);
    }

    #[test]
    fn round_trip_call_contract_function() {
        let sender = Keypair::new().pubkey();
        let destination_chain = "ethereum";
        let destination_contract_address =
            hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862").unwrap();
        let payload = bytes(100);

        let instruction = call_contract(
            crate::id(),
            sender,
            destination_chain,
            &destination_contract_address,
            &payload,
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
