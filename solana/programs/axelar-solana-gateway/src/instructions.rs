//! Instruction types

use std::fmt::Debug;

use axelar_solana_encoding::types::execute_data::{MerkleisedMessage, SigningVerifierSetInfo};
use axelar_solana_encoding::types::messages::Message;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use itertools::Itertools;
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::state::config::{RotationDelaySecs, VerifierSetEpoch};
use crate::state::verifier_set_tracker::VerifierSetHash;

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum GatewayInstruction {
    /// Processes incoming batch of ApproveMessage commands from Axelar
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Gateway ExecuteData PDA account
    /// 2. [] Verifier Setr Tracker PDA account (the one that signed the
    ///    ExecuteData)
    /// 3..N [WRITE] Gateway ApprovedCommand PDA accounts. All commands needs to
    ///         be `ApproveMessages`.
    ApproveMessage {
        /// The message that's to be approved
        message: MerkleisedMessage,
        /// The merkle root of the new message batch
        payload_merkle_root: [u8; 32],
    },

    /// Rotate signers for the Gateway Root Config PDA account.
    ///
    /// 0. [WRITE] Gateway Root Config PDA account
    /// 1. [] Verificatoin Session PDA account (should be valid)
    /// 2. [] The current verefier set tracker PDA (the one that signed the
    ///    verification payload)
    /// 3. [WRITE] The new verifier set tracker PDA (the one that needs to be
    ///    instantiated)
    /// 4. [WRITE, SIGNER] The payer for creating a new PAD
    /// 5. [] The system program
    RotateSigners {
        /// The merkle root of the new verifier set
        new_verifier_set_merkle_root: [u8; 32],
    },

    /// Represents the `CallContract` Axelar event.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Sender (origin) of the message)
    /// 1. [] Gateway Root Config PDA account
    CallContract {
        /// The name of the target blockchain.
        destination_chain: String,
        /// The address of the target contract in the destination blockchain.
        destination_contract_address: String,
        /// Contract call data.
        payload: Vec<u8>,
    },

    /// Represents the `CallContract` Axelar event. The contract call data is expected to be
    /// handled off-chain by uploading the data using the relayer API.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Sender (origin) of the message)
    /// 1. [] Gateway Root Config PDA account
    CallContractOffchainData {
        /// The name of the target blockchain.
        destination_chain: String,
        /// The address of the target contract in the destination blockchain.
        destination_contract_address: String,
        /// Hash of the contract call data, to be uploaded off-chain through the relayer API.
        payload_hash: [u8; 32],
    },

    /// Initializes the Gateway configuration PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [SIGNER] Gateway's Upgrade Authority account
    /// 2. [] Gateway's ProgramData account
    /// 3. [WRITE] Gateway Root Config PDA account
    /// 4. [] System Program account
    /// 5..N [WRITE] uninitialized VerifierSetTracker PDA accounts
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
    /// 2. [WRITE] Message Payload PDA account
    /// 3. [] System Program account
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
    /// 2. offset + bytes.len() is greater than the account size.
    /// 3. SIGNER is not the authority for the Message Payload account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Funding account and authority for the Message Payload account.
    /// 1. [] Gateway Root PDA account
    /// 2. [WRITE] Message Payload PDA account
    WriteMessagePayload {
        /// Offset at which to write the given bytes.
        offset: usize,
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
    /// 2. [WRITE] Message Payload PDA account
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
    /// 2. [WRITE] Message Payload PDA account
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

/// Configuration parameters for initializing the axelar-solana gateway
#[derive(Debug, Clone, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct InitializeConfig {
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// initial signer sets
    /// The order is important:
    /// - first element == oldest entry
    /// - last element == latest entry
    pub initial_signer_sets: Vec<VerifierSetHash>,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// The gateway operator.
    pub operator: Pubkey,
    /// how many n epochs do we consider valid
    pub previous_verifier_retention: VerifierSetEpoch,
}

/// Creates a [`GatewayInstruction::ApproveMessages`] instruction.
#[allow(clippy::too_many_arguments)]
pub fn approve_messages(
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
pub fn call_contract(
    gateway_program_id: Pubkey,
    gateway_root_pda: Pubkey,
    sender: Pubkey,
    destination_chain: String,
    destination_contract_address: String,
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
        program_id: gateway_program_id,
        accounts,
        data,
    })
}

/// Creates a [`CallContractOffchainData`] instruction.
pub fn call_contract_offchain_data(
    gateway_program_id: Pubkey,
    gateway_root_pda: Pubkey,
    sender: Pubkey,
    destination_chain: String,
    destination_contract_address: String,
    payload_hash: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::CallContractOffchainData {
        destination_chain,
        destination_contract_address,
        payload_hash,
    })?;

    let accounts = vec![
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
    ];

    Ok(Instruction {
        program_id: gateway_program_id,
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeConfig`] instruction.
#[allow(clippy::too_many_arguments)]
pub fn initialize_config(
    payer: Pubkey,
    upgrade_authority: Pubkey,
    domain_separator: [u8; 32],
    initial_signer_sets: Vec<(VerifierSetHash, Pubkey)>,
    minimum_rotation_delay: RotationDelaySecs,
    operator: Pubkey,
    previous_verifier_retention: VerifierSetEpoch,
    gateway_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let gateway_program_data =
        solana_program::bpf_loader_upgradeable::get_program_data_address(&crate::ID);

    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(upgrade_authority, true),
        AccountMeta::new_readonly(gateway_program_data, false),
        AccountMeta::new(gateway_config_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    initial_signer_sets.iter().for_each(|(_hash, pda)| {
        accounts.push(AccountMeta {
            pubkey: *pda,
            is_signer: false,
            is_writable: true,
        })
    });

    let data = to_vec(&GatewayInstruction::InitializeConfig(InitializeConfig {
        domain_separator,
        initial_signer_sets: initial_signer_sets.into_iter().map(|x| (x.0)).collect_vec(),
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
pub fn initialize_payload_verification_session(
    payer: Pubkey,
    gateway_config_pda: Pubkey,
    payload_merkle_root: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let (verification_session_pda, _) =
        crate::get_signature_verification_pda(&gateway_config_pda, &payload_merkle_root);

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
pub fn verify_signature(
    gateway_config_pda: Pubkey,
    verifier_set_tracker_pda: Pubkey,
    payload_merkle_root: [u8; 32],
    verifier_info: SigningVerifierSetInfo,
) -> Result<Instruction, ProgramError> {
    let (verification_session_pda, _bump) =
        crate::get_signature_verification_pda(&gateway_config_pda, &payload_merkle_root);

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

/// Creates a [`GatewayInstructon::ValidateMessage`] instruction.
pub fn validate_message(
    incoming_message_pda: &Pubkey,
    signing_pda: &Pubkey,
    message: Message,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*incoming_message_pda, false),
        AccountMeta::new_readonly(*signing_pda, true),
    ];

    let data = borsh::to_vec(&GatewayInstruction::ValidateMessage { message })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeMessagePayload`] instruction.
pub fn initialize_message_payload(
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    command_id: [u8; 32],
    buffer_size: u64,
) -> Result<Instruction, ProgramError> {
    let (message_payload_pda, _) =
        crate::find_message_payload_pda(gateway_root_pda, command_id, payer);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
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
pub fn write_message_payload(
    gateway_root_pda: Pubkey,
    authority: Pubkey,
    command_id: [u8; 32],
    bytes: &[u8],
    offset: usize,
) -> Result<Instruction, ProgramError> {
    let (message_payload_pda, _) =
        crate::find_message_payload_pda(gateway_root_pda, command_id, authority);
    let accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
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
pub fn commit_message_payload(
    gateway_root_pda: Pubkey,
    authority: Pubkey,
    command_id: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let (message_payload_pda, _) =
        crate::find_message_payload_pda(gateway_root_pda, command_id, authority);

    let accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
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
pub fn close_message_payload(
    gateway_root_pda: Pubkey,
    authority: Pubkey,
    command_id: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let (message_payload_pda, _) =
        crate::find_message_payload_pda(gateway_root_pda, command_id, authority);
    let accounts = vec![
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
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
