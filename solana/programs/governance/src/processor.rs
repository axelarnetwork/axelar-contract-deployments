//! Program instructions processor.

use alloy_sol_types::SolType;
use axelar_rkyv_encoding::hasher::solana::SolanaKeccak256Hasher;
use axelar_rkyv_encoding::hasher::{AxelarRkyv256Hasher, Hash256};
use axelar_rkyv_encoding::types::GmpMetadata;
use governance_gmp::GovernanceCommand::{
    ApproveOperatorProposal, CancelOperatorApproval, CancelTimeLockProposal,
    ScheduleTimeLockProposal,
};
use governance_gmp::GovernanceCommandPayload;
use program_utils::{check_rkyv_initialized_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use crate::instructions::GovernanceInstruction;
use crate::state::{ArchivedGovernanceConfig, GovernanceConfig};
use crate::{check_program_account, seed_prefixes};

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    ///
    /// # Errors
    ///
    /// A `ProgramError` containing the error that occurred is returned. Log
    /// messages are also generated with more detailed information.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        instruction_data: &[u8],
    ) -> ProgramResult {
        check_program_account(*program_id)?;

        let governance_instruction =
            GovernanceInstruction::from_bytes(instruction_data).map_err(|err| {
                msg!("Could not decode program input data: {}", err);
                ProgramError::InvalidArgument
            })?;

        match governance_instruction {
            GovernanceInstruction::InitializeConfig(governance_config) => {
                process_init_config(program_id, accounts, governance_config)
            }
            GovernanceInstruction::GovernanceGmpPayload { payload, metadata } => {
                process_gmp(program_id, accounts, &payload, &metadata)
            }
        }
    }
}

fn process_gmp(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    payload: &[u8],
    metadata: &GmpMetadata,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let _payer = next_account_info(accounts_iter)?;
    let root_pda = next_account_info(accounts_iter)?;

    let account_data = root_pda.try_borrow_data()?;

    let governance_config = check_rkyv_initialized_pda::<GovernanceConfig>(
        program_id,
        root_pda,
        account_data.as_ref(),
    )?;
    assert_valid_governance_root_pda(governance_config.bump, root_pda.key);
    ensure_authorized_gmp_command(governance_config, metadata)?;

    let GovernanceCommandPayload { command, .. } =
        GovernanceCommandPayload::abi_decode(payload, true).map_err(|err| {
            msg!("Cannot abi decode GovernanceCommandPayload: {}", err);
            ProgramError::InvalidArgument
        })?;

    // Todo, to implement on each match branch emit events, doings calls, etc ...
    match command {
        ScheduleTimeLockProposal => msg!("Executing ScheduleTimeLockProposal !"),
        CancelTimeLockProposal => msg!("Executing CancelTimeLockProposal !"),
        ApproveOperatorProposal => msg!("Executing ApproveOperatorProposal !"),
        CancelOperatorApproval => msg!("Executing CancelOperatorApproval !"),
        _ => (),
    }
    Ok(())
}

fn ensure_authorized_gmp_command(
    config: &ArchivedGovernanceConfig,
    meta: &GmpMetadata,
) -> Result<(), ProgramError> {
    // Ensure the incoming address matches stored configuration.
    let address_hash = hash(meta.source_address.as_bytes());
    if address_hash.0 != config.address_hash {
        msg!(
            "Incoming governance GMP message came with non authorized address: {}",
            meta.source_address
        );
        return Err(ProgramError::InvalidArgument);
    }

    // Ensure the incoming chain matches stored configuration.
    let chain = meta.cross_chain_id.chain();
    let chain_hash = hash(chain.as_bytes());
    if chain_hash.0 != config.chain_hash {
        msg!(
            "Incoming governance GMP message came with non authorized chain: {}",
            chain
        );
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

fn hash(data: &[u8]) -> Hash256 {
    let mut hasher = SolanaKeccak256Hasher::default();
    hasher.hash(data);
    hasher.result()
}

fn process_init_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    governance_config: GovernanceConfig,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let root_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    let bump = governance_config.bump;

    // Check: Gateway Config account uses the canonical bump.
    assert_valid_governance_root_pda(bump, root_pda.key);

    // Check: PDA Account is not initialized
    root_pda.check_uninitialized_pda()?;

    program_utils::init_rkyv_pda::<{ GovernanceConfig::LEN }, _>(
        payer,
        root_pda,
        program_id,
        system_account,
        governance_config,
        &[seed_prefixes::GOVERNANCE_CONFIG, &[bump]],
    )?;

    Ok(())
}

/// Assert that the governance PDA has been derived correctly
///
/// # Panics
///
/// This is early check and can panic the program.
#[inline]
pub fn assert_valid_governance_root_pda(bump: u8, expected_pubkey: &Pubkey) {
    #[allow(clippy::expect_used)]
    let derived_pubkey =
        Pubkey::create_program_address(&[seed_prefixes::GOVERNANCE_CONFIG, &[bump]], &crate::ID)
            .expect("invalid bump for the root config pda");

    assert_eq!(
        &derived_pubkey, expected_pubkey,
        "invalid gateway root config pda"
    );
}
