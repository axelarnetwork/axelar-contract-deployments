//! Instruction types

use std::fmt::Debug;

use axelar_solana_encoding::types::execute_data::{MerkleisedMessage, SigningVerifierSetInfo};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use itertools::Itertools;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::axelar_auth_weighted::{RotationDelaySecs, SignerSetEpoch};
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
        /// Bump for the message pda
        incoming_message_pda_bump: u8,
    },
    // todo ix for writing message contents into the PDA
    //--
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
        /// The bump for the new verifier set tracked PDA
        new_verifier_set_bump: u8,
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

    /// Initializes the Gateway configuration PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Gateway Root Config PDA account
    /// 2. [] System Program account
    /// 3..N [WRITE] uninitialized VerifierSetTracker PDA accounts
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
        /// Buffer account PDA bump seed
        bump_seed: u8,
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
    pub initial_signer_sets: Vec<(VerifierSetHash, u8)>,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// The gateway operator.
    pub operator: Pubkey,
    /// how many n epochs do we consider valid
    pub previous_signers_retention: SignerSetEpoch,
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
    incoming_message_pda_bump: u8,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::ApproveMessage {
        message,
        payload_merkle_root,
        incoming_message_pda_bump,
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
    new_verifier_set_bump: u8,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::RotateSigners {
        new_verifier_set_merkle_root,
        new_verifier_set_bump,
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
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeConfig`] instruction.
pub fn initialize_config(
    payer: Pubkey,
    domain_separator: [u8; 32],
    initial_signer_sets: Vec<(VerifierSetHash, Pubkey, u8)>,
    minimum_rotation_delay: RotationDelaySecs,
    operator: Pubkey,
    previous_signers_retention: SignerSetEpoch,
    gateway_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(gateway_config_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    initial_signer_sets.iter().for_each(|(_hash, pda, _)| {
        accounts.push(AccountMeta {
            pubkey: *pda,
            is_signer: false,
            is_writable: true,
        })
    });

    let data = to_vec(&GatewayInstruction::InitializeConfig(InitializeConfig {
        domain_separator,
        initial_signer_sets: initial_signer_sets
            .into_iter()
            .map(|x| (x.0, x.2))
            .collect_vec(),
        minimum_rotation_delay,
        operator,
        previous_signers_retention,
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
    let (verification_session_pda, bump_seed) =
        crate::get_signature_verification_pda(&gateway_config_pda, &payload_merkle_root);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_config_pda, false),
        AccountMeta::new(verification_session_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let data = to_vec(&GatewayInstruction::InitializePayloadVerificationSession {
        payload_merkle_root,
        bump_seed,
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
