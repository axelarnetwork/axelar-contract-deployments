//! This module holds all proposal related operations logic
//! Ideally, all proposal validations and calculations logic
//! must reside here.
//!
//! If we need to do any proposal validation outside the proposal
//! struct representation, like in the [`crate::processor`], it's
//! fine, but we should at least encapsulate such logic in a function
//! so the processor can use it from this module.

use std::error::Error;

use crate::sol_types::SolanaAccountMetadata;
use crate::{seed_prefixes, ID};
use program_utils::ValidPDA;
use program_utils::{
    check_rkyv_initialized_pda, checked_from_u256_le_bytes_to_u64, current_time, transfer_lamports,
};
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::keccak::hashv;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

type Uint256 = [u8; 32];
type Hash = [u8; 32];

/// This represents a proposal at the moment of it's storage. As we are using
/// RKYV as de/se technology, this represents the write model, while the read
/// model is implemented at [`ArchivedExecutableProposal`], which is generated
/// by RKYV derive traits.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
#[repr(C)]
#[allow(clippy::module_name_repetitions)]
pub struct ExecutableProposal {
    /// Represent the le bytes containing unix timestamp from when the proposal
    /// can be executed.
    eta: u64,
    /// The bump seed for the proposal PDA.
    bump: u8,
    /// The bump seed for the operator managed proposal PDA.
    managed_bump: u8,
}

impl ExecutableProposal {
    /// Approximated minimum length of the proposal. This allows RKYV
    /// to optimize pre-allocated space.
    const MIN_LEN: usize =
        core::mem::size_of::<u64>() + core::mem::size_of::<u8>() + core::mem::size_of::<u8>();
    /// Creates a new proposal. Currently only the timelock value (ETA) is
    /// stored.
    #[must_use]
    pub const fn new(eta: u64, bump: u8, managed_bump: u8) -> Self {
        Self {
            eta,
            bump,
            managed_bump,
        }
    }

    /// Calculates the hash for the proposal.
    ///
    /// # Arguments
    ///
    /// * `target` - The target program's public key.
    /// * `call_data` - The call data required to execute the proposal.
    /// * `native_value` - The native value associated with the proposal.
    ///
    /// # Returns
    ///
    /// A 32-byte array representing the hash of the proposal.
    ///
    /// # Panics
    ///
    /// Panics if the serialization of the call data fails. But it shouldn't.
    #[must_use]
    pub fn calculate_hash(
        target: &Pubkey,
        call_data: &ExecuteProposalCallData,
        native_value: &Uint256,
    ) -> Hash {
        let sol_accounts_ser = rkyv::to_bytes::<_, 0>(&call_data.solana_accounts)
            .expect("Solana accounts serialization failed");
        let native_value_ser =
            rkyv::to_bytes::<_, 0>(&call_data.solana_native_value_receiver_account)
                .expect("Solana native value receiver account serialization failed");
        let call_data_ser = &call_data.call_data;

        hashv(&[
            &target.to_bytes(),
            sol_accounts_ser.as_ref(),
            native_value_ser.as_ref(),
            call_data_ser,
            native_value,
        ])
        .to_bytes()
    }

    /// Derives the program-derived address (PDA) for a given hash.
    ///
    /// # Arguments
    ///
    /// * `hash` - A reference to a 32-byte array representing the hash.
    ///
    /// # Returns
    ///
    /// A tuple containing the derived `Pubkey` and the bump seed.
    #[must_use]
    pub fn pda(hash: &Hash) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seed_prefixes::PROPOSAL_PDA, hash], &ID)
    }

    /// Ensures that the provided PDA matches the expected PDA derived from the
    /// proposal hash and bump seed.
    ///
    /// # Arguments
    ///
    /// * `pda` - A reference to the provided PDA.
    /// * `proposal_hash` - A reference to a 32-byte array representing the
    ///   proposal hash.
    /// * `bump` - The bump seed for the PDA.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns a `ProgramError` if the derived PDA does not match the provided
    /// one.
    pub fn load_and_ensure_correct_proposal_pda(
        pda: &AccountInfo<'_>,
        proposal_hash: &[u8; 32],
    ) -> Result<u8, ProgramError> {
        // Check the proposal PDA exists and is initialized.
        if !pda.is_initialized_pda(&crate::id()) {
            msg!("Proposal PDA is not initialized");
            return Err(ProgramError::InvalidArgument);
        }

        let acc_data = pda.data.borrow();
        let proposal = ArchivedExecutableProposal::load_from(&crate::ID, pda, acc_data.as_ref())?;

        Self::ensure_correct_proposal_pda(pda, proposal_hash, proposal.bump())
    }

    /// Ensures that the provided PDA matches the expected PDA derived from the
    /// proposal hash and bump seed.
    ///
    /// # Arguments
    ///
    /// * `pda` - A reference to the provided PDA.
    /// * `proposal_hash` - A reference to a 32-byte array representing the
    ///   proposal hash.
    /// * `proposal_bump` - The bump seed for the PDA.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns a `ProgramError` if the derived PDA does not match the provided
    /// one.
    pub fn ensure_correct_proposal_pda(
        pda: &AccountInfo<'_>,
        proposal_hash: &[u8; 32],
        proposal_bump: u8,
    ) -> Result<u8, ProgramError> {
        let calculated_pda = Pubkey::create_program_address(
            &[seed_prefixes::PROPOSAL_PDA, proposal_hash, &[proposal_bump]],
            &crate::ID,
        )?;
        if calculated_pda != *pda.key {
            msg!("Derived proposal PDA does not match provided one");
            return Err(ProgramError::InvalidArgument);
        }
        Ok(proposal_bump)
    }

    /// Stores the proposal in the specified PDA.
    ///
    /// # Arguments
    ///
    /// * `system_account` - The system account.
    /// * `payer` - The account paying for the storage.
    /// * `proposal_pda` - The PDA where the proposal will be stored.
    /// * `hash` - The hash of the proposal.
    /// * `bump` - The bump seed for the PDA.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    ///
    /// # Errors
    ///
    /// An error if the account was already created.
    pub fn store<'a>(
        self,
        system_account: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        proposal_pda: &AccountInfo<'a>,
        hash: &[u8; 32],
        bump: u8,
    ) -> Result<(), ProgramError> {
        program_utils::init_rkyv_pda::<{ Self::MIN_LEN }, _>(
            payer,
            proposal_pda,
            &ID,
            system_account,
            self,
            &[seed_prefixes::PROPOSAL_PDA, hash, &[bump]],
        )
    }

    /// Returns the ETA (estimated time of arrival) of the proposal.
    ///
    /// # Returns
    ///
    /// A `u64` representing the ETA (unix timestamp).
    #[must_use]
    pub const fn eta(&self) -> u64 {
        self.eta
    }
}

impl ArchivedExecutableProposal {
    /// Loads an `ArchivedExecutableProposal` from the given account data.
    ///
    /// # Arguments
    ///
    /// * `program_id` - The program ID.
    /// * `account` - The account information.
    /// * `acc_data` - The account data.
    ///
    /// # Returns
    ///
    /// A result containing a reference to the `ArchivedExecutableProposal` or a
    /// `ProgramError`.
    ///
    /// # Errors
    ///
    /// A program error if the account data is not properly initialized.
    pub fn load_from<'a>(
        program_id: &Pubkey,
        account: &'a AccountInfo<'_>,
        acc_data: &'a [u8],
    ) -> Result<&'a Self, ProgramError> {
        //todo!("Check the pda was properly derived");
        check_rkyv_initialized_pda::<ExecutableProposal>(program_id, account, acc_data)
    }

    /// Checks if the proposal is unlocked.
    ///
    /// # Returns
    ///
    /// A result containing a boolean indicating whether the proposal is
    /// unlocked or a `ProgramError`.
    ///
    /// # Errors
    /// program error if the current time is not available.
    pub fn is_unlocked(&self) -> Result<bool, ProgramError> {
        Ok(current_time()? >= self.eta)
    }

    /// Executes the proposal by invoking the target program.
    /// This function will only execute the proposal if the proposal is
    /// unlocked and any other security checks pass.
    ///
    /// # Arguments
    ///
    /// * `target_program_accounts` - The target program accounts.
    /// * `config_pda` - The config PDA account information.
    /// * `config_pda_bump` - The config PDA bump value.
    /// * `target_address` - The target address.
    /// * `call_data` - The execute proposal call data.
    /// * `target_native_value_account_info` - The target native value account
    ///   information.
    /// * `native_value` - The native value.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    ///
    /// # Errors
    ///
    /// An error if the execution fails.
    #[allow(clippy::too_many_arguments)] // Todo: Yeah, but fix it.
    pub fn checked_execute(
        &self,
        target_program_accounts: &[AccountInfo<'_>],
        config_pda: &AccountInfo<'_>,
        config_pda_bump: u8,
        target_address: Pubkey,
        call_data: ExecuteProposalCallData,
        target_native_value_account_info: Option<&AccountInfo<'_>>,
        native_value: u64,
    ) -> Result<(), ProgramError> {
        if !self.is_unlocked()? {
            // Todo add in the err message WHEN should be able to execute.
            msg!("Proposal ETA needs to be respected");
            return Err(ProgramError::InvalidArgument);
        }

        Self::execute(
            target_program_accounts,
            config_pda,
            config_pda_bump,
            target_address,
            call_data,
            target_native_value_account_info,
            native_value,
        )
    }

    #[allow(clippy::too_many_arguments)] // Todo: Yeah, but fix it.
    fn execute(
        target_program_accounts: &[AccountInfo<'_>],
        config_pda: &AccountInfo<'_>,
        config_pda_bump: u8,
        target_address: Pubkey,
        call_data: ExecuteProposalCallData,
        target_native_value_account_info: Option<&AccountInfo<'_>>,
        native_value: u64,
    ) -> Result<(), ProgramError> {
        if native_value > 0 {
            let target_native_value_account_info = target_native_value_account_info
                .ok_or(ProgramError::InvalidArgument)
                .map_err(|err| {
                    msg!("Failed to get target native value account info: {}", err);
                    err
                })?;
            transfer_lamports(config_pda, target_native_value_account_info, native_value)?;
        }

        let accounts = call_data
            .solana_accounts
            .iter()
            .map(AccountMeta::from)
            .collect::<Vec<AccountMeta>>();

        // Invoke the target program.
        solana_program::program::invoke_signed(
            &Instruction {
                program_id: target_address,
                accounts,
                data: call_data.call_data,
            },
            target_program_accounts,
            &[&[seed_prefixes::GOVERNANCE_CONFIG, &[config_pda_bump]]],
        )
    }

    /// Executes the proposal without checking if it is unlocked.
    ///
    /// # Arguments
    ///
    /// * `target_program_accounts` - The accounts required by the target
    ///   program.
    /// * `config_pda` - The account info for the governance config PDA.
    /// * `config_pda_bump` - The bump seed for the governance config PDA.
    /// * `target_address` - The address of the target program.
    /// * `call_data` - The call data for the target program.
    /// * `target_native_value_account_info` - The account info for the target
    ///   native value account (optional).
    /// * `native_value` - The native value to transfer (optional).
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns a `ProgramError` if the execution fails.
    #[allow(clippy::too_many_arguments)] // Todo: Yeah, but fix it.
    pub fn unchecked_execute(
        &self,
        target_program_accounts: &[AccountInfo<'_>],
        config_pda: &AccountInfo<'_>,
        config_pda_bump: u8,
        target_address: Pubkey,
        call_data: ExecuteProposalCallData,
        target_native_value_account_info: Option<&AccountInfo<'_>>,
        native_value: u64,
    ) -> Result<(), ProgramError> {
        Self::execute(
            target_program_accounts,
            config_pda,
            config_pda_bump,
            target_address,
            call_data,
            target_native_value_account_info,
            native_value,
        )
    }

    /// Removes the proposal by closing the PDA and transferring the remaining
    /// lamports.
    ///
    /// # Arguments
    ///
    /// * `pda_to_close` - The PDA to be closed.
    /// * `lamport_destination` - The account to receive the remaining lamports.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns a `ProgramError` if the account closure fails.
    pub fn remove(
        pda_to_close: &AccountInfo<'_>,
        lamport_destination: &AccountInfo<'_>,
    ) -> Result<(), ProgramError> {
        program_utils::close_pda(lamport_destination, pda_to_close)
    }

    /// Returns the ETA (estimated time of arrival) of the proposal.
    ///
    /// # Returns
    ///
    /// A `u64` representing the ETA (unix timestamp).
    #[must_use]
    pub const fn eta(&self) -> u64 {
        self.eta
    }

    /// Returns the proposal bump seed.
    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }

    /// Returns the managed proposal bump seed.
    #[must_use]
    pub const fn managed_bump(&self) -> u8 {
        self.managed_bump
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
#[repr(C)]
/// Represents the data required to execute a proposal.
/// This struct is only used by the execute proposal instruction (See
/// [`crate::instructions::send_execute_proposal`]).
pub struct ExecuteProposalData {
    /// The target program address for the proposal, represented as a 32-byte
    /// array. Will be later converted to a [`solana_program::pubkey::Pubkey`].
    /// [`solana_program::pubkey::Pubkey`] when executing the proposal.
    pub target_address: [u8; 32],
    /// The data required to call the target program.
    pub call_data: ExecuteProposalCallData,
    /// A 32-byte array representing the native token U256 value (lamports)
    /// associated with the proposal. This is a U256 value and should be casted
    /// to u 64
    pub native_value: [u8; 32],
}

impl ExecuteProposalData {
    /// # Returns
    ///
    /// A new `ExecuteProposalData` instance.
    #[must_use]
    pub const fn new(
        target_address: [u8; 32],
        call_data: ExecuteProposalCallData,
        native_value: [u8; 32],
    ) -> Self {
        Self {
            target_address,
            call_data,
            native_value,
        }
    }

    /// Returns the target native value account info if the native value is
    /// greater than 0 and it can be found in the accounts slice. Otherwise,
    /// returns `None`.
    #[must_use]
    pub fn find_target_native_value_account_info<'a, 'b>(
        &self,
        accounts: &'a [AccountInfo<'b>],
    ) -> Option<&'a AccountInfo<'b>> {
        if self.native_value.iter().any(|&byte| byte > 0) {
            // Ensure the caller has provided the target account for the native token
            // transfer.
            let fund_target_pda = self
                .call_data
                .solana_native_value_receiver_account
                .as_ref()?;

            // Look for the funding target account in the accounts slice.
            accounts
                .iter()
                .find(|ai| ai.key.to_bytes() == fund_target_pda.pubkey.as_slice())
        } else {
            None
        }
    }

    /// Converts the internal native value to a `u64`.
    ///
    /// # Errors
    ///
    /// Returns a `ProgramError` and logs a message if the conversion fails.
    pub fn native_value(&self) -> Result<u64, ProgramError> {
        checked_from_u256_le_bytes_to_u64(&self.native_value).map_err(|err| {
            msg!("Failed to convert native value to u64: {}", err);
            ProgramError::InvalidArgument
        })
    }
}

/// Represents the call data required to execute the target program of a
/// proposal. This struct will be encoded as a byte array and added to the
/// [`governance_gmp::GovernanceCommandPayload::call_data`] instruction.
///
/// The Axelar governance infrastructure will use this struct to craft the GMP
/// message, meaning it should be aware of the Solana accounts needed by the
/// target program.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
#[repr(C)]
pub struct ExecuteProposalCallData {
    /// The Solana accounts metadata required for the target program in the
    /// moment of the proposal execution.
    ///
    /// In case the target program requires a native token transfer, the first
    /// account should be the target account the proposal should transfer the
    /// funds to.
    pub solana_accounts: Vec<SolanaAccountMetadata>,

    /// Apart from the [`Self::solana_accounts`] metadata, and in case the
    /// proposal requires a native token transfer to the target contract, the
    /// receiver account should be set here.
    pub solana_native_value_receiver_account: Option<SolanaAccountMetadata>,

    /// The call data required to execute the target program.
    pub call_data: Vec<u8>,
}

impl ExecuteProposalCallData {
    #[must_use]
    /// Creates a new `ExecuteProposalCallData` instance.
    ///
    /// # Arguments
    ///
    /// * `solana_accounts` - A vector of `SolanaAccountMetadata` representing
    ///   the Solana accounts metadata required for the target program.
    /// * `solana_native_value_receiver_account` - An optional
    ///   `SolanaAccountMetadata` representing the receiver account for native
    ///   token transfer.
    /// * `call_data` - A vector of bytes representing the call data required to
    ///   execute the target program.
    ///
    /// # Returns
    ///
    /// A new `ExecuteProposalCallData` instance.
    pub fn new(
        solana_accounts: Vec<SolanaAccountMetadata>,
        solana_native_value_receiver_account: Option<SolanaAccountMetadata>,
        call_data: Vec<u8>,
    ) -> Self {
        Self {
            solana_accounts,
            solana_native_value_receiver_account,
            call_data,
        }
    }

    /// Serializes Self into a byte array.
    ///
    /// # Errors
    ///
    /// If serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let bytes = rkyv::to_bytes::<_, 0>(self).map_err(Box::new)?;
        Ok(bytes.to_vec())
    }
}
