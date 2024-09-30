//! Main instructions for the governance contract.

use std::error::Error;

use axelar_rkyv_encoding::types::GmpMetadata;
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use crate::state::GovernanceConfig;

/// Instructions supported by the governance program.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum GovernanceInstruction {
    /// Initializes the governance configuration PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Governance Root Config PDA account
    /// 2. [] System Program account
    InitializeConfig(GovernanceConfig),

    /// A GMP instruction.
    ///
    /// 0. [signer] The address of payer / sender
    /// 1. [] governance root pda
    /// 2. [] ITS root pda
    /// 3..N Accounts depend on the inner ITS instruction.
    GovernanceGmpPayload {
        /// The GMP message metadata
        metadata: GmpMetadata,
        /// The GMP payload, abi encoded expected
        payload: Vec<u8>,
    },
}

impl GovernanceInstruction {
    /// Serializes the instruction into a byte array.
    ///
    /// # Errors
    ///
    /// If serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let bytes = rkyv::to_bytes::<_, 0>(self).map_err(Box::new)?;
        Ok(bytes.to_vec())
    }

    /// Deserializes the instruction from a byte array.
    ///
    /// # Errors
    ///
    /// If deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // SAFETY:
        // - The byte slice represents an archived object
        // - The root of the object is stored at the end of the slice
        let ix = unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }.map_err(Box::new)?;
        Ok(ix)
    }
}

/// Creates a [`GovernanceInstruction::InitializeConfig`] instruction.
/// # Errors
///
/// See [`ProgramError`] variants.
pub fn initialize_config(
    payer: &Pubkey,
    config: &GovernanceConfig,
    config_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = GovernanceInstruction::InitializeConfig(config.clone())
        .to_bytes()
        .map_err(|err| {
            msg!("unable to encode GovernanceInstruction {}", err.to_string());
            ProgramError::InvalidArgument
        })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GovernanceInstruction::GovernanceGmpPayload`] instruction.
/// # Errors
///
/// See [`ProgramError`] variants.
pub fn send_gmp_governance_message(
    payer: &Pubkey,
    config_pda: &Pubkey,
    gov_instruction: &GovernanceInstruction,
) -> Result<Instruction, ProgramError> {
    let accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = gov_instruction.to_bytes().map_err(|err| {
        msg!("unable to encode GovernanceInstruction: {}", err);
        ProgramError::InvalidArgument
    })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
