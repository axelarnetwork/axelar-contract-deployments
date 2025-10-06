//! Instruction types

use core::fmt::Debug;

use anchor_discriminators_macros::InstructionDiscriminator;
use axelar_solana_encoding::types::execute_data::{MerkleisedMessage, SigningVerifierSetInfo};
use axelar_solana_encoding::types::messages::Message;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::get_gateway_root_config_pda;
use crate::state::config::{RotationDelaySecs, VerifierSetEpoch};
use crate::state::verifier_set_tracker::VerifierSetHash;

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, InstructionDiscriminator)]
pub enum GatewayInstruction {
    /// Processes incoming batch of `ApproveMessage` commands from Axelar
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE, SIGNER] Payer account
    /// 2. [] Verification Session PDA account (should be valid)
    /// 3. [WRITE] Incoming Message PDA account
    /// 4. [] System Program account
    ApproveMessage {
        /// The message that's to be approved
        message: MerkleisedMessage,
        /// The merkle root of the new message batch
        payload_merkle_root: [u8; 32],
    },

    /// Rotate signers for the Gateway Root Config PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE] Gateway Root Config PDA account
    /// 1. [] Verification Session PDA account (should be valid)
    /// 2. [] The current verifier set tracker PDA (the one that signed the
    ///    verification payload)
    /// 3. [WRITE] The new verifier set tracker PDA (the one that needs to be
    ///    instantiated)
    /// 4. [WRITE, SIGNER] The payer for creating a new PDA
    /// 5. [] The system program
    /// 6. [SIGNER] (Optional) Operator account
    RotateSigners {
        /// The merkle root of the new verifier set
        new_verifier_set_merkle_root: [u8; 32],
    },

    /// Represents the `CallContract` Axelar event.
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Sender (origin) of the message, program id
    /// 1. [SIGNER] PDA created by the `sender`, works as authorization token for a given program id
    /// 2. [] Gateway Root Config PDA account
    CallContract {
        /// The name of the target blockchain.
        destination_chain: String,
        /// The address of the target contract in the destination blockchain.
        destination_contract_address: String,
        /// Contract call data.
        payload: Vec<u8>,
        /// The pda bump for the signing PDA
        signing_pda_bump: u8,
    },

    /// Initializes the Gateway configuration PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [SIGNER] Gateway's Upgrade Authority account
    /// 2. [] Gateway's `ProgramData` account
    /// 3. [WRITE] Gateway Root Config PDA account
    /// 4. [] System Program account
    /// 5. [WRITE] uninitialized `VerifierSetTracker` PDA account
    InitializeConfig(InitializeConfig),

    /// Initializes a verification session for a given Payload root.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [] Gateway Root Config PDA account
    /// 2. [WRITE] Verification session PDA buffer account
    /// 3. [] System Program account
    InitializePayloadVerificationSession {
        /// The Merkle root for the Payload being verified.
        payload_merkle_root: [u8; 32],
    },

    /// Verifies a signature within a Payload verification session
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Verification session PDA buffer account
    /// 2. [] Verifier Setr Tracker PDA account (the one that signed the
    ///    Payload's Merkle root)
    VerifySignature {
        /// The Merkle root for the Payload being verified.
        payload_merkle_root: [u8; 32],
        /// Information about the merkelised verifier set entry + the signature
        verifier_info: SigningVerifierSetInfo,
    },

    /// Initializes a Message Payload PDA account.
    ///
    /// This instruction will revert if the account already exists.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account, which becomes the authority for the Message Payload account.
    /// 1. [] Gateway Root PDA account
    /// 2. [] Incoming Message PDA account
    /// 3. [WRITE] Message Payload PDA account
    /// 4. [] System Program account
    InitializeMessagePayload {
        /// The number of bytes to allocate for the new message payload account
        buffer_size: u64,
        /// Message's command id
        command_id: [u8; 32],
    },

    /// Write message payload parts into the Message Payload PDA account.
    ///
    /// This instruction will revert on the following cases
    /// 1. Message payload account is already committed.
    /// 2. offset + `bytes.len()` is greater than the account size.
    /// 3. SIGNER is not the authority for the Message Payload account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Funding account and authority for the Message Payload account.
    /// 1. [] Gateway Root PDA account
    /// 2. [] Incoming Message PDA account
    /// 3. [WRITE] Message Payload PDA account
    WriteMessagePayload {
        /// Offset at which to write the given bytes.
        offset: u64,
        /// Serialized `execute_data` data.
        bytes: Vec<u8>,
        /// Message's command id
        command_id: [u8; 32],
    },

    /// Finalizes the writing phase for a Message Payload PDA buffer
    /// account and writes the calculated hash into its metadata
    /// section.
    ///
    /// This instruction will revert on the following circumstances:
    /// 1. The message payload account is already finalized.
    /// 2. The message payload account already had the payload hash calculated
    ///    and persisted.
    /// 3. SIGNER is not the authority for the Message Payload account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Funding account and authority for the Message Payload account.
    /// 1. [] Gateway Root PDA account
    /// 2. [] Incoming Message PDA account
    /// 3. [WRITE] Message Payload PDA account
    CommitMessagePayload {
        /// Message's command id
        command_id: [u8; 32],
    },

    /// Closes the message payload account and reclaim its lamports.
    ///
    /// This instruction will revert on the following circumstances:
    /// 1. SIGNER is not the authority for the Message Payload account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Funding account and authority for the Message Payload account.
    /// 1. [] Gateway Root PDA account
    /// 2. [] Incoming Message PDA account
    /// 3. [WRITE] Message Payload PDA account
    CloseMessagePayload {
        /// Message's command id
        command_id: [u8; 32],
    },

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
    /// 3. [SIGNER] PDA signer account (caller). Derived from the destination
    ///    program id.
    ValidateMessage {
        /// The Message that we want to approve
        message: Message,
    },

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

/// Represents an initial verifier set with its hash and PDA
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct InitialVerifierSet {
    /// The hash of the verifier set
    pub hash: VerifierSetHash,
    /// The PDA for the verifier set tracker
    pub pda: Pubkey,
}

/// Configuration parameters for initializing the axelar-solana gateway
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct InitializeConfig {
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// initial verifier set
    pub initial_verifier_set: InitialVerifierSet,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// The gateway operator.
    pub operator: Pubkey,
    /// how many n epochs do we consider valid
    pub previous_verifier_retention: VerifierSetEpoch,
}

/// Creates a [`GatewayInstruction::ApproveMessages`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
#[allow(clippy::too_many_arguments)]
pub fn approve_message(
    message: MerkleisedMessage,
    payload_merkle_root: [u8; 32],
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    verification_session_pda: Pubkey,
    incoming_message_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::ApproveMessage {
        message,
        payload_merkle_root,
    })?;

    let accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(verification_session_pda, false),
        AccountMeta::new(incoming_message_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::RotateSigners`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
#[allow(clippy::too_many_arguments)]
pub fn rotate_signers(
    gateway_root_pda: Pubkey,
    verification_session_account: Pubkey,
    current_verifier_set_tracker_pda: Pubkey,
    new_verifier_set_tracker_pda: Pubkey,
    payer: Pubkey,
    operator: Option<Pubkey>,
    new_verifier_set_merkle_root: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::RotateSigners {
        new_verifier_set_merkle_root,
    })?;

    let mut accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new_readonly(verification_session_account, false),
        AccountMeta::new_readonly(current_verifier_set_tracker_pda, false),
        AccountMeta::new(new_verifier_set_tracker_pda, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    if let Some(operator) = operator {
        accounts.push(AccountMeta::new(operator, true));
    }

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`CallContract`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
#[allow(clippy::too_many_arguments)]
pub fn call_contract(
    gateway_program_id: Pubkey,
    gateway_root_pda: Pubkey,
    sender: Pubkey,
    sender_call_contract_pda: Option<(Pubkey, u8)>,
    destination_chain: String,
    destination_contract_address: String,
    payload: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::CallContract {
        destination_chain,
        destination_contract_address,
        payload,
        signing_pda_bump: sender_call_contract_pda.map_or(0, |(_, bump)| bump),
    })?;

    let accounts = vec![
        AccountMeta::new_readonly(sender, sender_call_contract_pda.is_none()),
        AccountMeta::new_readonly(
            sender_call_contract_pda.map_or(crate::ID, |(pda, _)| pda),
            sender_call_contract_pda.is_some(),
        ),
        AccountMeta::new_readonly(gateway_root_pda, false),
    ];

    Ok(Instruction {
        program_id: gateway_program_id,
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeConfig`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
#[allow(clippy::too_many_arguments)]
pub fn initialize_config(
    payer: Pubkey,
    upgrade_authority: Pubkey,
    domain_separator: [u8; 32],
    initial_verifier_set: InitialVerifierSet,
    minimum_rotation_delay: RotationDelaySecs,
    operator: Pubkey,
    previous_verifier_retention: VerifierSetEpoch,
    gateway_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let gateway_program_data =
        solana_program::bpf_loader_upgradeable::get_program_data_address(&crate::ID);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(upgrade_authority, true),
        AccountMeta::new_readonly(gateway_program_data, false),
        AccountMeta::new(gateway_config_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new(initial_verifier_set.pda, false),
    ];

    let data = to_vec(&GatewayInstruction::InitializeConfig(InitializeConfig {
        domain_separator,
        initial_verifier_set,
        minimum_rotation_delay,
        operator,
        previous_verifier_retention,
    }))?;
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializePayloadVerificationSession`]
/// instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails..
pub fn initialize_payload_verification_session(
    payer: Pubkey,
    gateway_config_pda: Pubkey,
    payload_merkle_root: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let (verification_session_pda, _) = crate::get_signature_verification_pda(&payload_merkle_root);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_config_pda, false),
        AccountMeta::new(verification_session_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let data = to_vec(&GatewayInstruction::InitializePayloadVerificationSession {
        payload_merkle_root,
    })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::VerifySignature`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError`] if serialization of the [`GatewayInstruction::VerifySignature`]
/// instruction fails.
pub fn verify_signature(
    gateway_config_pda: Pubkey,
    verifier_set_tracker_pda: Pubkey,
    payload_merkle_root: [u8; 32],
    verifier_info: SigningVerifierSetInfo,
) -> Result<Instruction, ProgramError> {
    let (verification_session_pda, _bump) =
        crate::get_signature_verification_pda(&payload_merkle_root);

    let accounts = vec![
        AccountMeta::new_readonly(gateway_config_pda, false),
        AccountMeta::new(verification_session_pda, false),
        AccountMeta::new_readonly(verifier_set_tracker_pda, false),
    ];

    let data = to_vec(&GatewayInstruction::VerifySignature {
        payload_merkle_root,
        verifier_info,
    })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::ValidateMessage`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
pub fn validate_message(
    incoming_message_pda: &Pubkey,
    signing_pda: &Pubkey,
    message: Message,
) -> Result<Instruction, ProgramError> {
    let gateway_root_pda = get_gateway_root_config_pda().0;

    let accounts = vec![
        AccountMeta::new(*incoming_message_pda, false),
        AccountMeta::new_readonly(*signing_pda, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
    ];

    let data = borsh::to_vec(&GatewayInstruction::ValidateMessage { message })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeMessagePayload`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
pub fn initialize_message_payload(
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    command_id: [u8; 32],
    buffer_size: u64,
) -> Result<Instruction, ProgramError> {
    let (incoming_message_pda, _) = crate::get_incoming_message_pda(&command_id);
    let (message_payload_pda, _) = crate::find_message_payload_pda(incoming_message_pda, payer);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(incoming_message_pda, false),
        AccountMeta::new(message_payload_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let instruction = GatewayInstruction::InitializeMessagePayload {
        buffer_size,
        command_id,
    };

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::WriteMessagePayload`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.t
pub fn write_message_payload(
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    command_id: [u8; 32],
    bytes: &[u8],
    offset: u64,
) -> Result<Instruction, ProgramError> {
    let (incoming_message_pda, _) = crate::get_incoming_message_pda(&command_id);
    let (message_payload_pda, _) = crate::find_message_payload_pda(incoming_message_pda, payer);
    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(incoming_message_pda, false),
        AccountMeta::new(message_payload_pda, false),
    ];
    let instruction = GatewayInstruction::WriteMessagePayload {
        offset,
        bytes: bytes.to_vec(),
        command_id,
    };
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::CommitMessagePayload`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
pub fn commit_message_payload(
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    command_id: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let (incoming_message_pda, _) = crate::get_incoming_message_pda(&command_id);
    let (message_payload_pda, _) = crate::find_message_payload_pda(incoming_message_pda, payer);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(incoming_message_pda, false),
        AccountMeta::new(message_payload_pda, false),
    ];

    let instruction = GatewayInstruction::CommitMessagePayload { command_id };
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstrucon::CloseMessagePayload`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
pub fn close_message_payload(
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    command_id: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let (incoming_message_pda, _) = crate::get_incoming_message_pda(&command_id);
    let (message_payload_pda, _) = crate::find_message_payload_pda(incoming_message_pda, payer);
    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(incoming_message_pda, false),
        AccountMeta::new(message_payload_pda, false),
    ];
    let instruction = GatewayInstruction::CloseMessagePayload { command_id };
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::TransferOperatorship`] instruction.
///
/// # Errors
///
/// Returns a [`ProgramError::BorshIoError`] if the instruction serialization fails.
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
