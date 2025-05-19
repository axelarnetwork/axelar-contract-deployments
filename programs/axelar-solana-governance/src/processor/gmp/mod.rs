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
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use governance_gmp::GovernanceCommand::{
    ApproveOperatorProposal, CancelOperatorApproval, CancelTimeLockProposal,
    ScheduleTimeLockProposal,
};
use governance_gmp::GovernanceCommandPayload;
use program_utils::ValidPDA;
use solana_program::account_info::next_account_info;
use solana_program::account_info::AccountInfo;
use solana_program::keccak::hash;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::ensure_valid_governance_root_pda;
use crate::state::proposal::{ExecutableProposal, ExecuteProposalCallData};
use crate::state::GovernanceConfig;

mod approve_operator_proposal;
mod cancel_operator_approval;
mod cancel_time_lock_proposal;
mod schedule_time_lock_proposal;

/// The index of the first account that is expected to be passed to the
/// destination program.
pub const PROGRAM_ACCOUNTS_SPLIT_AT: usize = 4;

/// The index of the root PDA account in the accounts slice.
const ROOT_PDA_ACCOUNT_INDEX: usize = 2;

pub(crate) fn process(
    program_id: &Pubkey,
    gmp_ctx: ProcessGMPContext,
    gmp_accounts: &[AccountInfo<'_>],
) -> Result<(), ProgramError> {
    match gmp_ctx.cmd_payload.command {
        ScheduleTimeLockProposal => schedule_time_lock_proposal::process(gmp_ctx, gmp_accounts),
        CancelTimeLockProposal => cancel_time_lock_proposal::process(gmp_ctx, gmp_accounts),
        ApproveOperatorProposal => {
            approve_operator_proposal::process(gmp_ctx, program_id, gmp_accounts)
        }
        CancelOperatorApproval => cancel_operator_approval::process(gmp_ctx, gmp_accounts),
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
pub struct ProcessGMPContext {
    cmd_payload: GovernanceCommandPayload,
    target: Pubkey,
    proposal_hash: [u8; 32],
    minimum_eta_delay: u64,
}

impl ProcessGMPContext {
    /// Creates a new `ProcessGMPContext` from the given processor context.
    ///
    /// # Arguments
    ///
    /// * `_program_id` - The program ID.
    /// * `gw_accounts` - The gateway accounts.
    /// * `gmp_accounts` - The GMP accounts.
    /// * `message` - The message.
    ///
    /// # Errors
    ///
    /// Returns a `ProgramError` if any validation or decoding fails.
    pub fn new_from_processor_context(
        _program_id: &Pubkey,
        gw_accounts: &[AccountInfo<'_>],
        gmp_accounts: &[AccountInfo<'_>],
        message: &Message,
    ) -> Result<Self, ProgramError> {
        let root_pda = gmp_accounts
            .get(ROOT_PDA_ACCOUNT_INDEX)
            .ok_or(ProgramError::InvalidAccountData)
            .map_err(|err| {
                msg!("Cannot get root PDA account: {}", err);
                ProgramError::InvalidAccountData
            })?;

        let governance_config = root_pda.check_initialized_pda::<GovernanceConfig>(&crate::id())?;

        ensure_valid_governance_root_pda(governance_config.bump, root_pda.key)?;
        ensure_authorized_gmp_command(&governance_config, message)?;

        let gw_accounts_iter = &mut gw_accounts.iter();
        let _gateway_approved_message_pda = next_account_info(gw_accounts_iter)?;
        let payload_account = next_account_info(gw_accounts_iter)?;

        let payload_account_data = payload_account.try_borrow_data()?;
        let message_payload: ImmutMessagePayload<'_> = (**payload_account_data).try_into()?;

        let cmd_payload = payload_conversions::decode_payload(message_payload.raw_payload)?;

        let target = payload_conversions::decode_payload_target(&cmd_payload.target)?;

        let execute_proposal_call_data =
            payload_conversions::decode_payload_call_data(&cmd_payload.call_data)?;

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
        })
    }
}

/// A module to convert the payload data of a governance GMP command.
pub mod payload_conversions {

    use governance_gmp::alloy_primitives::Bytes;

    use super::*;

    /// Decodes the payload of a governance GMP command.
    ///
    /// # Errors
    ///
    /// A `ProgramError` is returned if the payload cannot be deserialized.
    pub fn decode_payload(raw_payload: &[u8]) -> Result<GovernanceCommandPayload, ProgramError> {
        GovernanceCommandPayload::abi_decode(raw_payload, true).map_err(|err| {
            msg!("Cannot abi decode GovernanceCommandPayload: {}", err);
            ProgramError::InvalidArgument
        })
    }

    /// Decodes the target address from the payload.
    ///
    /// # Errors
    ///
    /// A `ProgramError` is returned if the target address cannot be deserialized.
    pub fn decode_payload_target(payload_target_addr: &Bytes) -> Result<Pubkey, ProgramError> {
        let target: [u8; 32] = payload_target_addr.to_vec().try_into().map_err(|_err| {
            msg!("Cannot cast incoming target address for governance gmp command");
            ProgramError::InvalidArgument
        })?;
        Ok(Pubkey::from(target))
    }

    /// Decodes the call data from the payload.
    ///
    /// # Errors
    ///
    /// A `ProgramError` is returned if the call data cannot be deserialized.
    pub fn decode_payload_call_data(
        call_data: &Bytes,
    ) -> Result<ExecuteProposalCallData, ProgramError> {
        borsh::from_slice(call_data).map_err(|err| {
            msg!("Cannot deserialize ExecuteProposalCallData: {}", err);
            ProgramError::InvalidArgument
        })
    }
}

fn ensure_authorized_gmp_command(
    config: &GovernanceConfig,
    message: &Message,
) -> Result<(), ProgramError> {
    // Ensure the incoming address matches stored configuration.
    let address_hash = hash(message.source_address.as_bytes());
    if address_hash.0 != config.address_hash {
        msg!(
            "Incoming governance GMP message came with non authorized address: {}",
            message.source_address
        );
        return Err(ProgramError::InvalidArgument);
    }

    // Ensure the incoming chain matches stored configuration.
    let chain_hash = hash(message.cc_id.chain.as_bytes());
    if chain_hash.0 != config.chain_hash {
        msg!(
            "Incoming governance GMP message came with non authorized chain: {}",
            message.cc_id.chain
        );
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
