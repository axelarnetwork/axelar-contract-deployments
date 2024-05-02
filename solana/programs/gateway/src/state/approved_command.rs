//! Module for the GatewayApprovedCommand account type.

use std::mem::size_of;

use axelar_message_primitives::command::DecodedCommand;
use axelar_message_primitives::DestinationProgramId;
use borsh::{BorshDeserialize, BorshSerialize};
use num_traits::ToBytes;
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

/// Gateway Approved Command type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayApprovedCommand {
    /// Status of the command
    status: GatewayCommandStatus,
    /// The bump that was used to create the PDA
    pub bump: u8,
}

/// Differnet states of the command
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum GatewayCommandStatus {
    /// The Command has been executed.
    /// Maps to this [line in the Solidity Gateway](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/78fde453094074ca93ef7eea1e1395fba65ba4f6/contracts/AxelarGateway.sol#L525)
    ValidateContractCall(ValidateContractCall),
    /// The Command has been executed.
    /// And the command type was `TransferOperatorship`
    TransferOperatorship(TransferOperatorship),
}

/// After the command itself is marked as `Approved`, the command can be used
/// for CPI gateway::validateContractCall instruction.
/// This maps to [these lines in the Solidity Gateway](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/78fde453094074ca93ef7eea1e1395fba65ba4f6/contracts/AxelarGateway.sol#L636-L648)
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum ValidateContractCall {
    /// The state of the command before it has been approved
    Pending,
    /// The state of the command after it has been approved
    Approved,
    /// `ValidateContractCall` has been called and the command has been executed
    /// by the destination program.
    Executed,
}

/// Represents the state of a `TransferOperatorship` command that comes from the
/// Axelar network.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum TransferOperatorship {
    /// The state of the command before it has been approved
    Pending,
    /// `TransferOperatorship` has been called and the command has been executed
    Executed,
}

impl GatewayApprovedCommand {
    /// Returns an pending command.
    pub fn pending(bump: u8, command: &DecodedCommand) -> Self {
        let status = {
            match command {
                DecodedCommand::ApproveContractCall(_command) => {
                    GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Pending)
                }
                DecodedCommand::TransferOperatorship(_command) => {
                    GatewayCommandStatus::TransferOperatorship(TransferOperatorship::Pending)
                }
            }
        };
        Self { status, bump }
    }

    /// Makes sure that the attached account info is the expected one
    /// If successful verification: will update the status to `Executed`
    pub fn validate_contract_call(
        &mut self,
        command_id: &[u8; 32],
        destination_pubkey: &DestinationProgramId,
        caller: &AccountInfo<'_>,
    ) -> Result<(), ProgramError> {
        let (allowed_caller, _allowed_caller_bump) = destination_pubkey.signing_pda(command_id);
        if allowed_caller != *caller.key {
            return Err(GatewayError::MismatchedAllowedCallers.into());
        }

        if !caller.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !self.is_contract_call_approved() {
            return Err(GatewayError::GatewayCommandNotApproved.into());
        }

        self.status = GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Executed);

        Ok(())
    }

    /// Sets the command status as approved.
    pub fn set_ready_for_validate_contract_call(&mut self) -> Result<(), ProgramError> {
        if !matches!(
            self.status,
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Pending)
        ) {
            return Err(GatewayError::GatewayCommandStatusNotPending.into());
        }
        self.status = GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Approved);

        Ok(())
    }
    /// Sets the command status as executed.
    pub fn set_transfer_operatorship_executed(&mut self) -> Result<(), ProgramError> {
        if !matches!(
            self.status,
            GatewayCommandStatus::TransferOperatorship(TransferOperatorship::Pending)
        ) {
            return Err(GatewayError::GatewayCommandStatusNotPending.into());
        }
        self.status = GatewayCommandStatus::TransferOperatorship(TransferOperatorship::Executed);

        Ok(())
    }

    /// Returns `true` if this command was executed by the gateway.
    pub fn is_command_executed(&self) -> bool {
        matches!(
            self.status,
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Executed)
        ) || matches!(
            self.status,
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Approved)
        ) || matches!(
            self.status,
            GatewayCommandStatus::TransferOperatorship(TransferOperatorship::Executed)
        )
    }

    /// Returns `true` if this command was executed by the gatewy and the
    /// destination program has called the `validateContractCall` instruction.
    pub fn is_contract_call_validated(&self) -> bool {
        matches!(
            self.status,
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Executed)
        )
    }

    /// Returns `true` if this command was approved.
    pub fn is_contract_call_approved(&self) -> bool {
        matches!(
            self.status,
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Approved)
        )
    }

    /// Returns the status of this command.
    pub fn status(&self) -> &GatewayCommandStatus {
        &self.status
    }

    /// Finds a PDA for this account by hashing the parameters. Returns its
    /// Pubkey and bump.
    pub fn pda(
        gateway_root_pda: &Pubkey,
        command_params: &DecodedCommand,
    ) -> (Pubkey, u8, [u8; 32]) {
        let seeds_hash = Self::calculate_seed_hash(gateway_root_pda, command_params);

        let (pubkey, bump) = Pubkey::find_program_address(&[seeds_hash.as_ref()], &crate::ID);
        (pubkey, bump, seeds_hash)
    }

    /// Calculates the seed hash for the PDA of this account.
    pub fn calculate_seed_hash(
        gateway_root_pda: &Pubkey,
        command_params: &DecodedCommand,
    ) -> [u8; 32] {
        use solana_program::keccak::hashv;

        match command_params {
            DecodedCommand::ApproveContractCall(command_params) => {
                let (signing_pda_for_destination_pubkey, signing_pda_bump) = command_params
                    .destination_program
                    .signing_pda(&command_params.command_id);

                let seeds = &[
                    gateway_root_pda.as_ref(),
                    command_params.command_id.as_ref(),
                    command_params.source_chain.as_ref(),
                    command_params.source_address.as_ref(),
                    command_params.payload_hash.as_ref(),
                    signing_pda_for_destination_pubkey.as_ref(),
                    &[signing_pda_bump],
                ];

                // Hashing is necessary because otherwise the seeds would be too long
                hashv(seeds).to_bytes()
            }
            DecodedCommand::TransferOperatorship(command_params) => {
                let res = command_params
                    .operators
                    .iter()
                    .map(|operator| operator.omit_prefix())
                    .chain(command_params.weights.iter().map(|x| {
                        let mut bytes = [0; 32];
                        bytes[0..16].copy_from_slice(&x.to_be_bytes());
                        bytes
                    }))
                    .collect::<Vec<_>>();
                let mut seeds = res.iter().map(|x| x.as_ref()).collect::<Vec<_>>();
                seeds.push(gateway_root_pda.as_ref());
                let quorum = command_params.quorum.to_le_bytes();
                seeds.push(quorum.as_ref());

                // Hashing is necessary because otherwise the seeds would be too long
                hashv(seeds.as_ref()).to_bytes()
            }
        }
    }

    /// Asserts that the PDA for this account is valid.
    pub fn assert_valid_pda(&self, seed_hash: &[u8; 32], exppected_pubkey: &Pubkey) {
        let derived_pubkey = Pubkey::create_program_address(&[seed_hash, &[self.bump]], &crate::ID)
            .expect("invalid bump for the root pda");
        assert_eq!(
            &derived_pubkey, exppected_pubkey,
            "invalid pda for the gateway approved command"
        );
    }
}

impl Sealed for GatewayApprovedCommand {}

impl Pack for GatewayApprovedCommand {
    const LEN: usize = size_of::<GatewayApprovedCommand>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
