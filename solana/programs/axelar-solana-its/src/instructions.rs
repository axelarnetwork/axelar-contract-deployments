//! Instructions supported by the multicall program.

use std::error::Error;

use axelar_message_primitives::{DataPayload, DestinationProgramId};
use axelar_rkyv_encoding::types::GmpMetadata;
use gateway::hasher_impl;
use interchain_token_transfer_gmp::GMPPayload;
use rkyv::bytecheck::EnumCheckError;
use rkyv::validation::validators::DefaultValidatorError;
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::state::token_manager;

/// Instructions supported by the multicall program.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum InterchainTokenServiceInstruction {
    /// Initializes the interchain token service program.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writeable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [writeable] ITS root pda
    /// 3. [] system program id
    Initialize {
        /// The pda bump for the ITS root PDA
        pda_bump: u8,
    },

    /// A GMP Interchain Token Service instruction.
    ///
    /// 0. [writeable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] ITS root pda
    ///
    /// 3..N Accounts depend on the inner ITS instruction.
    ItsGmpPayload {
        /// The GMP metadata
        gmp_metadata: GmpMetadata,

        /// The GMP payload
        abi_payload: Vec<u8>,

        /// The PDA bumps for the ITS accounts
        bumps: Bumps,
    },
}

impl InterchainTokenServiceInstruction {
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
        let bytes = unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }.map_err(Box::new)?;

        Ok(bytes)
    }
}

impl ArchivedInterchainTokenServiceInstruction {
    /// Interprets the given slice as an archived instruction.
    ///
    /// # Errors
    ///
    /// If validation fails.
    pub fn from_archived_bytes(
        bytes: &[u8],
    ) -> Result<&Self, rkyv::validation::CheckArchiveError<EnumCheckError<u8>, DefaultValidatorError>>
    {
        rkyv::check_archived_root::<InterchainTokenServiceInstruction>(bytes)
    }
}

/// Convenience module with the indices of the accounts passed in the
/// [`ItsGmpPayload`] instruction (offset by the prefixed GMP accounts).
pub mod its_account_indices {
    /// The index of the system program account.
    pub const SYSTEM_PROGRAM_INDEX: usize = 0;

    /// The index of the ITS root PDA account.
    pub const ITS_ROOT_PDA_INDEX: usize = 1;

    /// The index of the token manager PDA account.
    pub const TOKEN_MANAGER_PDA_INDEX: usize = 2;

    /// The index of the token mint account.
    pub const TOKEN_MINT_INDEX: usize = 3;

    /// The index of the token manager ATA account.
    pub const TOKEN_MANAGER_ATA_INDEX: usize = 4;

    /// The index of the token program account.
    pub const TOKEN_PROGRAM_INDEX: usize = 5;

    /// The index of the associated token program account.
    pub const SPL_ASSOCIATED_TOKEN_ACCOUNT_INDEX: usize = 6;
}

/// Creates a [`InterchainTokenServiceInstruction::Initialize`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn initialize(
    payer: &Pubkey,
    gateway_root_pda: &Pubkey,
    its_root_pda: &(Pubkey, u8),
) -> Result<Instruction, ProgramError> {
    let instruction = InterchainTokenServiceInstruction::Initialize {
        pda_bump: its_root_pda.1,
    };

    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new(its_root_pda.0, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Bumps for the ITS PDA accounts.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Copy)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct Bumps {
    /// The bump for the ITS root PDA.
    pub its_root_pda_bump: u8,

    /// The bump for the interchain token PDA.
    pub interchain_token_pda_bump: u8,

    /// The bump for the token manager PDA.
    pub token_manager_pda_bump: u8,
}

/// Inputs for the [`its_gmp_payload`] function.
pub struct ItsGmpInstructionInputs {
    /// The payer account.
    pub payer: Pubkey,

    /// The PDA that tracks the approval of this message by the gateway program.
    pub gateway_approved_message_pda: Pubkey,

    /// The root PDA for the gateway program.
    pub gateway_root_pda: Pubkey,

    /// The Axelar GMP metadata.
    pub gmp_metadata: GmpMetadata,

    /// The ITS GMP payload.
    pub payload: GMPPayload,

    /// The token program required by the instruction (spl-token or
    /// spl-token-2022).
    pub token_program: Pubkey,

    /// The mint account required by the instruction. Hard requirement for
    /// `InterchainTransfer` instruction. Optional for `DeployTokenManager` and
    /// ignored by `DeployInterchainToken`.
    pub mint: Option<Pubkey>,

    /// Bumps used to derive the ITS accounts. If not set, the
    /// `find_program_address` is used which is more expensive.
    pub bumps: Option<Bumps>,
}

/// Creates a [`InterchainTokenServiceInstruction::ItsGmpPayload`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn its_gmp_payload(inputs: ItsGmpInstructionInputs) -> Result<Instruction, ProgramError> {
    let mut accounts = prefix_accounts(
        &inputs.payer,
        &inputs.gateway_approved_message_pda,
        &inputs.gateway_root_pda,
        &inputs.gmp_metadata,
    );
    let (mut its_accounts, bumps) = derive_its_accounts(
        &inputs.gateway_root_pda,
        &inputs.payload,
        inputs.token_program,
        inputs.mint,
        inputs.bumps,
    )?;

    accounts.append(&mut its_accounts);

    let data = InterchainTokenServiceInstruction::ItsGmpPayload {
        abi_payload: inputs.payload.encode(),
        gmp_metadata: inputs.gmp_metadata,
        bumps,
    }
    .to_bytes()
    .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

fn prefix_accounts(
    payer: &Pubkey,
    gateway_approved_message_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
    gmp_metadata: &GmpMetadata,
) -> Vec<AccountMeta> {
    let command_id = gmp_metadata.cross_chain_id.command_id(hasher_impl());
    let destination_program = DestinationProgramId(crate::id());
    let (gateway_approved_message_signing_pda, _) = destination_program.signing_pda(&command_id);

    vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*gateway_approved_message_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
    ]
}

pub(crate) fn derive_its_accounts(
    gateway_root_pda: &Pubkey,
    payload: &GMPPayload,
    token_program: Pubkey,
    mint: Option<Pubkey>,
    maybe_bumps: Option<Bumps>,
) -> Result<(Vec<AccountMeta>, Bumps), ProgramError> {
    let (maybe_its_root_pda_bump, maybe_interchain_token_pda_bump, maybe_token_manager_pda_bump) =
        maybe_bumps.map_or((None, None, None), |bumps| {
            (
                Some(bumps.its_root_pda_bump),
                Some(bumps.interchain_token_pda_bump),
                Some(bumps.token_manager_pda_bump),
            )
        });

    let token_id = payload.token_id();
    let (its_root_pda, its_root_pda_bump) =
        crate::its_root_pda(gateway_root_pda, maybe_its_root_pda_bump);
    let (interchain_token_pda, interchain_token_pda_bump) =
        crate::interchain_token_pda(&its_root_pda, token_id, maybe_interchain_token_pda_bump);
    let (token_manager_pda, token_manager_pda_bump) =
        crate::token_manager_pda(&interchain_token_pda, maybe_token_manager_pda_bump);
    let token_mint = try_retrieve_mint(&interchain_token_pda, payload, mint)?;

    if let GMPPayload::DeployInterchainToken(_) = payload {
        if token_program != spl_token_2022::id() {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    let mut accounts =
        derive_common_its_accounts(its_root_pda, token_mint, token_manager_pda, token_program);

    match payload {
        GMPPayload::InterchainTransfer(transfer_data) => {
            let destination_wallet = Pubkey::new_from_array(
                transfer_data
                    .destination_address
                    .as_ref()
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            );
            let destination_ata = get_associated_token_address_with_program_id(
                &destination_wallet,
                &token_mint,
                &token_program,
            );

            accounts.push(AccountMeta::new(destination_wallet, false));
            accounts.push(AccountMeta::new(destination_ata, false));

            if !transfer_data.data.is_empty() {
                let execute_data = DataPayload::decode(transfer_data.data.as_ref())
                    .map_err(|_err| ProgramError::InvalidInstructionData)?;

                accounts.extend(execute_data.account_meta().iter().cloned());
            }
        }
        GMPPayload::DeployInterchainToken(message) => {
            accounts.push(AccountMeta::new_readonly(sysvar::rent::id(), false));
            if message.minter.len() == axelar_rkyv_encoding::types::ED25519_PUBKEY_LEN {
                accounts.push(AccountMeta::new_readonly(
                    Pubkey::new_from_array(
                        message
                            .minter
                            .as_ref()
                            .try_into()
                            .map_err(|_err| ProgramError::InvalidInstructionData)?,
                    ),
                    false,
                ));
            }
        }
        GMPPayload::DeployTokenManager(_message) => {}
    };

    Ok((
        accounts,
        Bumps {
            its_root_pda_bump,
            interchain_token_pda_bump,
            token_manager_pda_bump,
        },
    ))
}

fn try_retrieve_mint(
    interchain_token_pda: &Pubkey,
    payload: &GMPPayload,
    maybe_mint: Option<Pubkey>,
) -> Result<Pubkey, ProgramError> {
    if let Some(mint) = maybe_mint {
        return Ok(mint);
    }

    match payload {
        GMPPayload::DeployTokenManager(message) => {
            let token_mint = token_manager::decode_params(message.params.as_ref())
                .map(|(_, token_mint)| Pubkey::try_from(token_mint.as_ref()))?
                .map_err(|_err| ProgramError::InvalidInstructionData)?;

            Ok(token_mint)
        }
        GMPPayload::InterchainTransfer(_transfer_data) => {
            maybe_mint.ok_or(ProgramError::InvalidInstructionData)
        }
        GMPPayload::DeployInterchainToken(_message) => Ok(*interchain_token_pda),
    }
}

fn derive_common_its_accounts(
    its_root_pda: Pubkey,
    mint_account: Pubkey,
    token_manager_pda: Pubkey,
    token_program: Pubkey,
) -> Vec<AccountMeta> {
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &mint_account,
        &token_program,
    );

    vec![
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint_account, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
    ]
}
