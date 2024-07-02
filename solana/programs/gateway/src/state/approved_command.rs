//! Module for the GatewayApprovedCommand account type.

use std::mem::size_of;

use axelar_message_primitives::DestinationProgramId;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

use crate::commands::{AxelarMessage, Command, CommandKind};
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
    /// The status of a single `ApprovedMessage` command
    /// Maps to this [line in the Solidity Gateway](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/78fde453094074ca93ef7eea1e1395fba65ba4f6/contracts/AxelarGateway.sol#L525)
    ApprovedMessage(ApprovedMessageStatus),
    /// The status of a single `RotateSigners` command
    RotateSigners(RotateSignersStatus),
}

/// After the command itself is marked as `Approved`, the command can be used
/// for CPI [`GatewayInstructon::ValidateMessage`] instruction.
/// This maps to [these lines in the Solidity Gateway](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/78fde453094074ca93ef7eea1e1395fba65ba4f6/contracts/AxelarGateway.sol#L636-L648)
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum ApprovedMessageStatus {
    /// The state of the command before it has been approved
    Pending,
    /// The state of the command after it has been approved
    Approved,
    /// [`GatewayInstructon::ValidateMessage`] has been called and the command
    /// has been executed by the destination program.
    Executed,
}

/// Represents the state of a `RotateSigners` command that comes from the
/// Axelar network.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum RotateSignersStatus {
    /// The state of the command before it has been approved
    Pending,
    /// `RotateSigners` has been called and the command has been executed
    Executed,
}

impl GatewayApprovedCommand {
    /// Returns an pending command.
    pub fn pending(bump: u8, command: &impl Command) -> Self {
        let status = {
            match command.kind() {
                CommandKind::ApproveMessage => {
                    GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Pending)
                }
                CommandKind::RotateSigner => {
                    GatewayCommandStatus::RotateSigners(RotateSignersStatus::Pending)
                }
            }
        };
        Self { status, bump }
    }

    /// Ensures that the command is valid (seed hash matches) and is in a
    /// pending state.
    pub fn command_valid_and_pending(
        self,
        gateway_root_pda: &Pubkey,
        command: &impl Command,
        message_account: &AccountInfo<'_>,
    ) -> Result<Option<Self>, ProgramError> {
        // Check: Current message account represents the current approved command.
        let seed_hash = GatewayApprovedCommand::calculate_seed_hash(gateway_root_pda, command);

        self.assert_valid_pda(&seed_hash, message_account.key);

        // https://github.com/axelarnetwork/cgp-spec/blob/c3010b9187ad9022dbba398525cf4ec35b75e7ae/solidity/contracts/AxelarGateway.sol#L103
        if !self.is_command_pending() {
            return Ok(None);
        }

        Ok(Some(self))
    }

    /// Makes sure that the attached account info is the expected one
    /// If successful verification: will update the status to `Executed`
    pub fn validate_message(
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
        if !self.is_command_approved() {
            return Err(GatewayError::GatewayCommandNotApproved.into());
        }

        self.status = GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Executed);

        Ok(())
    }

    /// Sets the command status as approved.
    pub fn set_ready_for_validate_message(&mut self) -> Result<(), ProgramError> {
        if !matches!(
            self.status,
            GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Pending)
        ) {
            return Err(GatewayError::GatewayCommandStatusNotPending.into());
        }
        self.status = GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Approved);

        Ok(())
    }
    /// Sets the command status as executed.
    pub fn set_signers_rotated_executed(&mut self) -> Result<(), ProgramError> {
        if !matches!(
            self.status,
            GatewayCommandStatus::RotateSigners(RotateSignersStatus::Pending)
        ) {
            return Err(GatewayError::GatewayCommandStatusNotPending.into());
        }
        self.status = GatewayCommandStatus::RotateSigners(RotateSignersStatus::Executed);

        Ok(())
    }

    /// returns `true` if this command was executed by the gateway.
    pub fn is_command_pending(&self) -> bool {
        matches!(
            self.status,
            GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Pending)
                | GatewayCommandStatus::RotateSigners(RotateSignersStatus::Pending)
        )
    }

    /// Returns `true` if this command was executed by the gateway.
    pub fn is_command_executed(&self) -> bool {
        matches!(
            self.status,
            GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Executed)
                | GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Approved)
                | GatewayCommandStatus::RotateSigners(RotateSignersStatus::Executed)
        )
    }

    /// Returns `true` if this command was executed by the gatewy and the
    /// destination program has called the
    /// [`GatewayInstructon::ValidateMessage`] instruction.
    pub fn is_validate_message_executed(&self) -> bool {
        matches!(
            self.status,
            GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Executed)
        )
    }

    /// Returns `true` if this command was approved. Done after the
    /// [`GatewayInstructon::ApproveMessages`] has been called.
    pub fn is_command_approved(&self) -> bool {
        matches!(
            self.status,
            GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Approved)
        )
    }

    /// Returns the status of this command.
    pub fn status(&self) -> &GatewayCommandStatus {
        &self.status
    }

    /// Finds a PDA for this account by hashing the parameters. Returns its
    /// Pubkey and bump.
    pub fn pda(gateway_root_pda: &Pubkey, command: &impl Command) -> (Pubkey, u8, [u8; 32]) {
        let seeds_hash = Self::calculate_seed_hash(gateway_root_pda, command);

        let (pubkey, bump) = Pubkey::find_program_address(&[seeds_hash.as_ref()], &crate::ID);
        (pubkey, bump, seeds_hash)
    }

    /// Calculates the seed hash for the PDA of this account.
    pub fn calculate_seed_hash(gateway_root_pda: &Pubkey, command: &impl Command) -> [u8; 32] {
        use solana_program::keccak::hashv;
        let mut signing_pda_buffer = [0u8; 33]; // 32 bytes for the public key + 1 for the bump
        let command_hash = command.hash();

        // TODO: Bubble this error up in the call tree
        if let Some(axelar_message) = command.axelar_message() {
            let (signing_pda_for_destination_program, signing_pda_bump) = axelar_message
                .destination_program()
                .expect("failed to infer signing PDA for the destination program")
                .signing_pda(&command_hash);
            signing_pda_buffer.copy_from_slice(signing_pda_for_destination_program.as_ref());
            signing_pda_buffer[32] = signing_pda_bump;
        }

        let seeds = vec![
            gateway_root_pda.as_ref(),
            command_hash.as_slice(),
            &signing_pda_buffer,
        ];

        // Hashing is necessary because otherwise the seeds would be too long
        hashv(&seeds).to_bytes()
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
