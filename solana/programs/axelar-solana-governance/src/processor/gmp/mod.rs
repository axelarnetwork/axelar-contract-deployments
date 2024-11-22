//! The processing parts of the incoming GMP flows of the governance program.
//!
//! # Invariants
//!
//! * All GMP commands are sent by the Axelar network to this governance
//!   program.
//! * It is enforced that only a pre-configured source address and blockchain
//!   can send GMP messages to this program.
//!
//! The payload data structure here is
//! [`governance_gmp::GovernanceCommandPayload`]. Which is adapted to accomplish
//! the [original eth implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/b5d0b7bdda0437fce983daffb776669437b809d0/contracts/governance/InterchainGovernance.sol#L134). Inside the
//! [`governance_gmp::GovernanceCommandPayload::call_data`] field of such
//! struct, the [`crate::proposal::ExecuteProposalCallData`]
//!
//! This is the main GMP governance processing unit. See sub-modules for
//! each GMP command processing logic.

use alloy_sol_types::SolType;
use axelar_executable_old::validate_with_gmp_metadata;
use axelar_rkyv_encoding::types::GmpMetadata;
use governance_gmp::GovernanceCommand::{
    ApproveOperatorProposal, CancelOperatorApproval, CancelTimeLockProposal,
    ScheduleTimeLockProposal,
};
use governance_gmp::GovernanceCommandPayload;
use program_utils::check_rkyv_initialized_pda;
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::{ensure_valid_governance_root_pda, hash};
use crate::state::proposal::{ExecutableProposal, ExecuteProposalCallData};
use crate::state::{ArchivedGovernanceConfig, GovernanceConfig};

mod approve_operator_proposal;
mod cancel_operator_approval;
mod cancel_time_lock_proposal;
mod schedule_time_lock_proposal;

/// The index of the first account that is expected to be passed to the
/// destination program.
const PROGRAM_ACCOUNTS_SPLIT_AT: usize = 4;

/// The index of the root PDA account in the accounts slice.
const ROOT_PDA_ACCOUNT_INDEX: usize = 2;

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    payload: &[u8],
    metadata: &GmpMetadata,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();

    let (gateway_accounts, gmp_accounts) =
        accounts_iter.as_slice().split_at(PROGRAM_ACCOUNTS_SPLIT_AT);
    validate_with_gmp_metadata(&crate::id(), gateway_accounts, metadata.clone(), payload)?;

    let root_pda = gmp_accounts
        .get(ROOT_PDA_ACCOUNT_INDEX)
        .ok_or(ProgramError::InvalidAccountData)
        .map_err(|err| {
            msg!("Cannot get root PDA account: {}", err);
            ProgramError::InvalidAccountData
        })?;

    let ctx =
        ProcessGMPContext::new_from_processor_context(program_id, root_pda, metadata, payload)?;

    match ctx.cmd_payload.command {
        ScheduleTimeLockProposal => schedule_time_lock_proposal::process(ctx, gmp_accounts),
        CancelTimeLockProposal => cancel_time_lock_proposal::process(ctx, gmp_accounts),
        ApproveOperatorProposal => {
            approve_operator_proposal::process(ctx, program_id, gmp_accounts)
        }
        CancelOperatorApproval => cancel_operator_approval::process(ctx, gmp_accounts),
        _ => {
            msg!("Governance GMP command cannot be processed");
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

/// A convenience struct to hold the context for processing a Governance GMP
/// command. This context can be used to process any GMP command.
///
/// Mandatory, it should be executed before any GMP command processing, as it
/// contains key validation logic.
struct ProcessGMPContext {
    cmd_payload: GovernanceCommandPayload,
    target: Pubkey,
    proposal_hash: [u8; 32],
    minimum_eta_delay: u64,
    execute_proposal_call_data: ExecuteProposalCallData,
}

impl ProcessGMPContext {
    fn new_from_processor_context(
        program_id: &Pubkey,
        root_pda: &AccountInfo<'_>,
        metadata: &GmpMetadata,
        payload: &[u8],
    ) -> Result<Self, ProgramError> {
        let account_data = root_pda.try_borrow_data()?;
        let governance_config = check_rkyv_initialized_pda::<GovernanceConfig>(
            program_id,
            root_pda,
            account_data.as_ref(),
        )?;

        ensure_valid_governance_root_pda(governance_config.bump, root_pda.key)?;
        ensure_authorized_gmp_command(governance_config, metadata)?;

        let cmd_payload = GovernanceCommandPayload::abi_decode(payload, true).map_err(|err| {
            msg!("Cannot abi decode GovernanceCommandPayload: {}", err);
            ProgramError::InvalidArgument
        })?;

        let target: [u8; 32] = cmd_payload.target.to_vec().try_into().map_err(|_err| {
            msg!("Cannot cast incoming target address for governance gmp command");
            ProgramError::InvalidArgument
        })?;
        let target = Pubkey::from(target);

        let execute_proposal_call_data: ExecuteProposalCallData =
            rkyv::from_bytes(&cmd_payload.call_data).map_err(|err| {
                msg!("Cannot deserialize ExecuteProposalCallData: {}", err);
                ProgramError::InvalidArgument
            })?;

        let proposal_hash = ExecutableProposal::calculate_hash(
            &target,
            &execute_proposal_call_data,
            &cmd_payload.native_value.to_le_bytes(),
        );

        Ok(Self {
            cmd_payload,
            proposal_hash,
            minimum_eta_delay: u64::from(governance_config.minimum_proposal_eta_delay),
            target,
            execute_proposal_call_data,
        })
    }
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
