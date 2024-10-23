//! Instruction types

use std::error::Error;
use std::fmt::Debug;

use axelar_rkyv_encoding::types::{ArchivedVerifierSet, VerifierSet};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use itertools::Itertools;
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::axelar_auth_weighted::{RotationDelaySecs, SignerSetEpoch};
use crate::commands::{CommandKind, MessageWrapper, OwnedCommand};
use crate::hasher_impl;
use crate::state::execute_data::{
    ApproveMessagesVariant, ExecuteDataVariant, RotateSignersVariant,
};
use crate::state::{GatewayApprovedCommand, GatewayExecuteData};

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
    ApproveMessages,

    /// Rotate signers for the Gateway Root Config PDA account.
    ///
    /// 0. [] Gateway ExecuteData PDA account
    /// 1. [] Verifier Setr Tracker PDA account (the one that signed the
    ///    ExecuteData)
    /// 2. [WRITE, SIGNER] new uninitialized VerifierSetTracker PDA account (the
    ///    one that needs to be initialized)
    /// 3. [WRITE, SIGNER] Funding account for the new VerifierSetTracker PDA
    /// 4. [] System Program account
    /// 5. Optional: [SIGNER] `Operator` that's stored in the gateway config
    ///    PDA.
    RotateSigners,

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
    InitializeConfig(InitializeConfig<(VerifierSetWrapper, PdaBump)>),

    /// Initializes an Approve Messages Execute Data PDA account.
    /// The Execute Data is a batch of commands that will be executed by the
    /// Execute instruction (separate step). The `execute_data` will be
    /// decoded on-chain to verify the data is correct and generate the proper
    /// hash, and store it in the Approve Messages Execute Data PDA account.
    ///
    /// It's expected that for each command in the batch, there is a
    /// corresponding `GatewayApprovedCommand` account. The sequence of
    /// which is initialized first is not important.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Approve Messages Execute Data PDA account
    /// 2. [] System Program account
    InitializeApproveMessagesExecuteData {
        /// The execute data that will be decoded.
        /// We decode it on-chain so we can verify the data is correct and
        /// generate the proper hash.
        execute_data: Vec<u8>,
    },

    /// Initializes a Rotate Signers Execute Data PDA account.
    /// The Execute Data is a batch of commands that will be executed by the
    /// Execute instruction (separate step). The `execute_data` will be
    /// decoded on-chain to verify the data is correct and generate the proper
    /// hash, and store it in the Rotate Signers Execute Data PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Rotate Signers Execute Data PDA account
    /// 2. [] System Program account
    InitializeRotateSignersExecuteData {
        /// The execute data that will be decoded.
        /// We decode it on-chain so we can verify the data is correct and
        /// generate the proper hash.
        execute_data: Vec<u8>,
    },

    /// Initializes a pending command.
    /// This instruction is used to initialize a command that will trackt he
    /// execution state of a command contained in a batch.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Gateway ApprovedCommand PDA account
    /// 2. [] Gateway Root Config PDA account
    /// 3. [] System Program account
    InitializePendingCommand(OwnedCommand),

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
    ValidateMessage(MessageWrapper),

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

    /// Initializes an `execute_data` PDA buffer account.
    ///
    /// This instruction will revert if the buffer account already exists.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [] Gateway Root Config PDA account
    /// 2. [WRITE] Execute Data PDA buffer account
    /// 3. [] System Program account
    InitializeExecuteDataBuffer {
        /// The number of bytes to allocate for the new buffer account
        buffer_size: u64,
        /// User provided seed for the buffer account PDA
        user_seed: [u8; 32],
        /// Buffer account PDA bump seed
        bump_seed: u8,
        /// The command kind that will be written into this buffer
        command_kind: CommandKind,
    },

    /// Write `execute_data` parts into the PDA buffer account.
    ///
    /// This instruction will revert on the following cases
    /// 1. Buffer account is already finalized
    /// 2. offset + bytes.len() is greater than the buffer size.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE] Execute Data PDA buffer account
    WriteExecuteDataBuffer {
        /// Offset at which to write the given bytes.
        offset: usize,
        /// Serialized `execute_data` data.
        bytes: Vec<u8>,
    },

    /// Finalizes the writing phase for an `execute_data` PDA buffer account and
    /// writes the calculated `Payload` hash into its metadata section.
    ///
    /// This instruction will revert on the following circumstances:
    /// 1. The buffer account is already finalized.
    /// 2. The buffer account already had the `execute_data` hash calculated and
    ///    persisted.
    /// 3. Instruction fails to decode the previously written `execute_data`
    ///    bytes.
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Execute Data PDA buffer account
    CommitPayloadHash {},

    /// Initializes the signature validation state machine.
    ///
    /// Requires that the `execute_data_buffer`'s write phase has been
    /// finalized.
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Execute Data PDA buffer account
    InitializeSignatureVerification {
        /// Merkle root for the `Proof`'s signatures.
        signature_merkle_root: [u8; 32],
    },

    /// Verifies a signature as part of the `execute_data` validation
    /// process.
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Execute Data PDA buffer account
    VerifySignature {
        /// The signature bytes.
        signature_bytes: Vec<u8>,
        /// The signer's public key bytes.
        public_key_bytes: Vec<u8>,
        /// The signer's weight.
        signer_weight: u128,
        /// The signer's position within the verifier set.
        signer_index: u8,
        /// This signatures's proof of inclusion in the signatures merkle tree.
        signature_merkle_proof: Vec<u8>,
    },

    /// Finalize an `execute_data` PDA buffer account.
    ///
    /// The `execute_data` will be decoded on-chain to verify the data
    /// is correct and generate the proper hash, and store it in the
    /// Approve Messages Execute Data PDA account.
    ///
    /// It's expected that for each command in the batch, there is a
    /// corresponding `GatewayApprovedCommand` account. The sequence of
    /// which is initialized first is not important.
    ///
    /// This instruction will revert if the buffer account is already
    /// finalized or if the signature verification didn't collect
    /// sufficient signer weight.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE] Execute Data PDA buffer account
    FinalizeExecuteDataBuffer {},
}

/// Configuration parameters for initializing the axelar-solana gateway
#[derive(Debug, Clone, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct InitializeConfig<T> {
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// initial signer sets
    /// The order is important:
    /// - first element == oldest entry
    /// - last element == latest entry
    pub initial_signer_sets: Vec<T>,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// The gateway operator.
    pub operator: Pubkey,
    /// how many n epochs do we consider valid
    pub previous_signers_retention: SignerSetEpoch,
}

type PdaBump = u8;
type InitializeConfigTransformation = (
    InitializeConfig<(VerifierSetWrapper, PdaBump)>,
    Vec<(Pubkey, PdaBump)>,
);

impl InitializeConfig<VerifierSetWrapper> {
    /// Convert  [`InitializeConfig`] to a type that can be submitted to the
    /// gateway by calculating the PDAs for the initial signers.
    pub fn with_verifier_set_bump(self) -> InitializeConfigTransformation {
        let (pdas, bumps): (Vec<(Pubkey, PdaBump)>, Vec<PdaBump>) = self
            .calculate_verifier_set_pdas()
            .into_iter()
            .map(|(pda, bump)| ((pda, bump), bump))
            .unzip();
        let initial_signer_sets = bumps
            .into_iter()
            .zip_eq(self.initial_signer_sets)
            .map(|(bump, set)| (set, bump))
            .collect_vec();
        (
            InitializeConfig {
                domain_separator: self.domain_separator,
                initial_signer_sets,
                minimum_rotation_delay: self.minimum_rotation_delay,
                operator: self.operator,
                previous_signers_retention: self.previous_signers_retention,
            },
            pdas,
        )
    }

    /// Calculate the PDAs and PDA bumps for the initial verifiers
    pub fn calculate_verifier_set_pdas(&self) -> Vec<(Pubkey, PdaBump)> {
        self.initial_signer_sets
            .iter()
            .map(|init_verifier_set| {
                let hash = init_verifier_set
                    .parse()
                    .expect("invalid Verifier set provided")
                    .hash(hasher_impl());
                let (pda, derived_bump) = crate::get_verifier_set_tracker_pda(&crate::id(), hash);
                (pda, derived_bump)
            })
            .collect_vec()
    }
}

/// Because [`axelar_rkyv_encoding::types::VerifierSet`] does not implement
/// borsh, we depend on the data to be encoded with rkyv. This is a wrapper that
/// implements borsh.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct VerifierSetWrapper {
    /// rkyv encoded [`axelar_rkyv_encoding::types::VerifierSet`]
    verifier_set: Vec<u8>,
}

impl VerifierSetWrapper {
    /// Encode the verifier set and initialize the wrapper
    pub fn new_from_verifier_set(
        verifier_set: VerifierSet,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            verifier_set: verifier_set.to_bytes()?,
        })
    }

    /// Decode the encoded verifier set
    pub fn parse(
        &self,
    ) -> Result<
        &ArchivedVerifierSet,
        axelar_rkyv_encoding::rkyv::validation::CheckArchiveError<
            axelar_rkyv_encoding::rkyv::bytecheck::StructCheckError,
            axelar_rkyv_encoding::rkyv::validation::validators::DefaultValidatorError,
        >,
    > {
        ArchivedVerifierSet::from_archived_bytes(&self.verifier_set)
    }
}

/// Creates a [`GatewayInstruction::ApproveMessages`] instruction.
pub fn approve_messages(
    execute_data_account: Pubkey,
    gateway_root_pda: Pubkey,
    command_accounts: &[Pubkey],
    verifier_set_tracker_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::ApproveMessages)?;

    let mut accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(execute_data_account, false),
        AccountMeta::new_readonly(verifier_set_tracker_pda, false),
    ];

    // Message accounts needs to be writable so we can set them as processed.
    accounts.extend(
        command_accounts
            .iter()
            .map(|key| AccountMeta::new(*key, false)),
    );

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::RotateSigners`] instruction.
pub fn rotate_signers(
    execute_data_account: Pubkey,
    gateway_root_pda: Pubkey,
    operator: Option<Pubkey>,
    current_verifier_set_tracker_pda: Pubkey,
    new_verifier_set_tracker_pda: Pubkey,
    payer: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::RotateSigners)?;

    let mut accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new_readonly(execute_data_account, false),
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

fn initialize_gateway_execute_data<T>(
    payer: Pubkey,
    gateway_root_pda: Pubkey,
    domain_separator: &[u8; 32],
    instruction: GatewayInstruction,
) -> Result<(Instruction, GatewayExecuteData<T>), ProgramError>
where
    T: ExecuteDataVariant,
{
    let raw_execute_data = match &instruction {
        GatewayInstruction::InitializeApproveMessagesExecuteData { execute_data } => execute_data,
        GatewayInstruction::InitializeRotateSignersExecuteData { execute_data } => execute_data,
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // We decode the data off-chain so we can find its PDA.
    let decoded_execute_data =
        GatewayExecuteData::new(raw_execute_data, &gateway_root_pda, domain_separator)?;
    let (execute_data_pda, _) = crate::get_execute_data_pda(
        &gateway_root_pda,
        &decoded_execute_data.hash_decoded_contents(),
    );

    // We store the raw data so we can verify it on-chain.
    let data = to_vec(&instruction)?;

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
/// Creates a [`GatewayInstruction::InitializeApproveMessagesExecuteData`]
/// instruction.
pub fn initialize_approve_messages_execute_data(
    payer: Pubkey,
    gateway_root_pda: Pubkey,
    domain_separator: &[u8; 32],
    // The encoded data that will be decoded on-chain.
    raw_execute_data: &[u8],
) -> Result<(Instruction, GatewayExecuteData<ApproveMessagesVariant>), ProgramError> {
    let instruction = GatewayInstruction::InitializeApproveMessagesExecuteData {
        execute_data: raw_execute_data.to_vec(),
    };

    initialize_gateway_execute_data(payer, gateway_root_pda, domain_separator, instruction)
}

/// Creates a [`GatewayInstruction::InitializeRotateSignersExecuteData`]
/// instruction.
pub fn initialize_rotate_signers_execute_data(
    payer: Pubkey,
    gateway_root_pda: Pubkey,
    domain_separator: &[u8; 32],
    // The encoded data that will be decoded on-chain.
    raw_execute_data: &[u8],
) -> Result<(Instruction, GatewayExecuteData<RotateSignersVariant>), ProgramError> {
    let instruction = GatewayInstruction::InitializeRotateSignersExecuteData {
        execute_data: raw_execute_data.to_vec(),
    };

    initialize_gateway_execute_data(payer, gateway_root_pda, domain_separator, instruction)
}

/// Creates a [`GatewayInstruction::InitializeConfig`] instruction.
pub fn initialize_config(
    payer: Pubkey,
    config: InitializeConfig<VerifierSetWrapper>,
    gateway_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(gateway_config_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    let (config, with_verifier_set_bump) = config.with_verifier_set_bump();
    with_verifier_set_bump.into_iter().for_each(|(pda, _)| {
        accounts.push(AccountMeta {
            pubkey: pda,
            is_signer: false,
            is_writable: true,
        })
    });

    let data = to_vec(&GatewayInstruction::InitializeConfig(config))?;
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
    message_wrapper: MessageWrapper,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*approved_message_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*caller, true),
    ];

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

/// Creates a [`GatewayInstruction::InitializeExecuteDataBuffer`] instruction.
pub fn initialize_execute_data_buffer(
    gateway_root_pda: Pubkey,
    payer: Pubkey,
    buffer_size: u64,
    user_seed: [u8; 32],
    command_kind: CommandKind,
) -> Result<Instruction, ProgramError> {
    let (buffer_pda, bump_seed) = crate::get_execute_data_pda(&gateway_root_pda, &user_seed);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(buffer_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let instruction = GatewayInstruction::InitializeExecuteDataBuffer {
        buffer_size,
        user_seed,
        bump_seed,
        command_kind,
    };

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::WriteExecuteDataBuffer`] instruction.
pub fn write_execute_data_buffer(
    gateway_root_pda: Pubkey,
    user_seed: &[u8; 32],
    bump_seed: u8,
    bytes: &[u8],
    offset: usize,
) -> Result<Instruction, ProgramError> {
    let buffer_pda = crate::create_execute_data_pda(&gateway_root_pda, user_seed, bump_seed)?;
    let accounts = vec![AccountMeta::new(buffer_pda, false)];
    let instruction = GatewayInstruction::WriteExecuteDataBuffer {
        offset,
        bytes: bytes.to_vec(),
    };
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::CommitPayloadHash`] instruction.
pub fn commit_payload_hash(
    gateway_root_pda: Pubkey,
    user_seed: &[u8; 32],
    bump_seed: u8,
) -> Result<Instruction, ProgramError> {
    let buffer_pda = crate::create_execute_data_pda(&gateway_root_pda, user_seed, bump_seed)?;
    let accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(buffer_pda, false),
    ];
    let instruction = GatewayInstruction::CommitPayloadHash {};
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::InitializeSignatureVerification`]
/// instruction.
pub fn initialize_signature_verification(
    gateway_root_pda: Pubkey,
    user_seed: &[u8; 32],
    bump_seed: u8,
    signature_merkle_root: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let buffer_pda = crate::create_execute_data_pda(&gateway_root_pda, user_seed, bump_seed)?;
    let accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(buffer_pda, false),
    ];
    let instruction = GatewayInstruction::InitializeSignatureVerification {
        signature_merkle_root,
    };
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::VerifySignature`] instruction.
#[allow(clippy::too_many_arguments)]
pub fn verify_signature(
    gateway_root_pda: Pubkey,
    user_seed: &[u8; 32],
    bump_seed: u8,
    signature_bytes: Vec<u8>,
    public_key_bytes: Vec<u8>,
    signer_weight: u128,
    signer_index: u8,
    signature_merkle_proof: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let buffer_pda = crate::create_execute_data_pda(&gateway_root_pda, user_seed, bump_seed)?;
    let accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(buffer_pda, false),
    ];

    let instruction = GatewayInstruction::VerifySignature {
        signature_bytes,
        public_key_bytes,
        signer_weight,
        signer_index,
        signature_merkle_proof,
    };
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

/// Creates a [`GatewayInstruction::FinalizeExecuteDataBuffer`] instruction.
pub fn finalize_execute_data_buffer(
    gateway_root_pda: Pubkey,
    user_seed: &[u8; 32],
    bump_seed: u8,
) -> Result<Instruction, ProgramError> {
    let buffer_pda = crate::create_execute_data_pda(&gateway_root_pda, user_seed, bump_seed)?;
    let accounts = vec![AccountMeta::new(buffer_pda, false)];
    let instruction = GatewayInstruction::FinalizeExecuteDataBuffer {};
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data: borsh::to_vec(&instruction)?,
    })
}

#[cfg(test)]
pub mod tests {

    use borsh::from_slice;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;

    use super::*;
    use crate::state::GatewayConfig;

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
        let verifier_set_tracker_pda = Pubkey::new_unique();
        let instruction = approve_messages(
            execute_data_account,
            gateway_root_pda,
            &approved_message_accounts,
            verifier_set_tracker_pda,
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
        let destination_chain = "ethereum".to_owned();
        let destination_contract_address = "2F43DDFf564Fb260dbD783D55fc6E4c70Be18862".to_owned();
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
        let destination_chain = "ethereum".to_owned();
        let destination_contract_address = "2F43DDFf564Fb260dbD783D55fc6E4c70Be18862".to_owned();
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
