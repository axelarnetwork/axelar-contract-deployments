use bitflags::bitflags;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction, system_program};

use super::{Processor, ToBytes};
use crate::commands::CommandKind;
use crate::error::GatewayError;
use crate::seed_prefixes;
use crate::state::execute_data::{
    ApproveMessagesVariant, ExecuteDataVariant, RotateSignersVariant,
};
use crate::state::{GatewayConfig, GatewayExecuteData};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_execute_data<T>(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        execute_data: Vec<u8>,
    ) -> Result<(), ProgramError>
    where
        GatewayExecuteData<T>: ToBytes,
        T: ExecuteDataVariant,
    {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let execute_data_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Gateway Root PDA is initialized.
        let domain_separator = gateway_root_pda
            .check_initialized_pda::<GatewayConfig>(program_id)?
            .domain_separator;

        let Ok(execute_data) =
            GatewayExecuteData::<T>::new(&execute_data, gateway_root_pda.key, &domain_separator)
        else {
            msg!("Failed to deserialize execute_data bytes");
            return Err(ProgramError::InvalidAccountData);
        };

        // Check: Execute Data account is not initialized.
        if let Err(err) = execute_data_account.check_uninitialized_pda() {
            msg!("Execute Datat PDA already initialized");
            return Err(err);
        }
        // Check: Execute Data PDA is correctly derived
        crate::assert_valid_execute_data_pda(
            &execute_data,
            gateway_root_pda.key,
            execute_data_account.key,
        );

        super::init_pda_with_dynamic_size(
            payer,
            execute_data_account,
            &[
                seed_prefixes::EXECUTE_DATA_SEED,
                gateway_root_pda.key.as_ref(),
                &execute_data.hash_decoded_contents(),
                &[execute_data.bump],
            ],
            &execute_data,
        )
    }

    /// Handles the
    /// [`crate::instructions::GatewayInstruction::InitializeExecuteDataBuffer`]
    /// instruction.
    pub fn process_initialize_execute_data_buffer(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        buffer_size: u64,
        user_seed: &[u8; 32],
        bump_seed: u8,
        command_kind: CommandKind,
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let buffer_account = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        assert!(payer.is_signer);
        assert!(payer.is_writable);
        assert!(!buffer_account.is_signer);
        assert!(buffer_account.is_writable);
        assert_eq!(buffer_account.lamports(), 0);
        assert!(system_program::check_id(system_program.key));

        // Check: Gateway Root PDA is initialized.
        gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        // Check: Buffer PDA can be derived from provided seeds.
        let buffer_pda =
            crate::create_execute_data_pda(gateway_root_pda.key, user_seed, bump_seed)?;
        if buffer_pda != *buffer_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Add an extra first byte to track buffer metadata.
        let adjusted_buffer_size = buffer_size
            .checked_add(1)
            .ok_or(ProgramError::AccountDataTooSmall)?;

        // Prepare the `create_account` instruction
        let lamports_required = Rent::get()?.minimum_balance(adjusted_buffer_size as usize);
        let create_pda_account_ix = system_instruction::create_account(
            payer.key,
            buffer_account.key,
            lamports_required,
            adjusted_buffer_size,
            program_id,
        );

        // Use the same seeds as `[crate::get_execute_data_pda]`, plus the bump seed.
        let signers_seeds = &[
            seed_prefixes::EXECUTE_DATA_SEED,
            gateway_root_pda.key.as_ref(),
            user_seed,
            &[bump_seed],
        ];

        // Create the empty buffer account.
        invoke_signed(
            &create_pda_account_ix,
            &[
                payer.clone(),
                buffer_account.clone(),
                system_program.clone(),
            ],
            &[signers_seeds],
        )?;

        // Set the metadata/flags in the buffer's first byte
        let mut buffer = buffer_account.try_borrow_mut_data()?;
        let first_byte = buffer
            .first_mut()
            .ok_or(ProgramError::AccountDataTooSmall)?;
        *first_byte = BufferMetadata::new_from_command_kind(command_kind).bits();

        Ok(())
    }

    /// Handles the
    /// [`crate::instructions::GatewayInstruction::WriteExecuteDataBuffer`]
    /// instruction.
    pub fn process_write_execute_data_buffer(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        bytes: &[u8],
        offset: usize,
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let buffer_account = next_account_info(accounts_iter)?;
        assert!(buffer_account.is_writable);
        assert_eq!(buffer_account.owner, program_id);

        let mut data = buffer_account.try_borrow_mut_data()?;

        // Split the finalization byte from the account data.
        if data.len() <= 1 {
            return Err(ProgramError::AccountDataTooSmall);
        };
        let (metadata, buffer) = data.split_at_mut(1);

        // Check: buffer account should not be finalized.
        if metadata
            .first()
            .and_then(|bits| BufferMetadata::from_bits(*bits))
            .ok_or(ProgramError::InvalidAccountData)?
            .is_finalized()
        {
            msg!("Buffer account is finalized");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: Write bounds
        let write_offset = offset.saturating_add(bytes.len());
        if buffer.len() < write_offset {
            msg!("Write overflow: {} < {}", buffer.len(), write_offset);
            return Err(ProgramError::AccountDataTooSmall);
        }

        buffer
            .get_mut(offset..write_offset)
            .ok_or(ProgramError::AccountDataTooSmall)?
            .copy_from_slice(bytes);

        Ok(())
    }

    /// Handles the
    /// [`crate::instructions::GatewayInstruction::FinalizeExecuteDataBuffer`]
    /// instruction.
    pub fn process_finalize_execute_data_buffer(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let buffer_account = next_account_info(accounts_iter)?;
        assert!(buffer_account.is_writable);
        assert_eq!(buffer_account.owner, program_id);

        // Check: Gateway Root PDA is initialized.
        let domain_separator = gateway_root_pda
            .check_initialized_pda::<GatewayConfig>(program_id)?
            .domain_separator;

        let mut data = buffer_account.try_borrow_mut_data()?;

        // Split the finalization byte from the account data.
        if data.len() <= 1 {
            return Err(ProgramError::AccountDataTooSmall);
        };
        let (metadata_bits, buffer) = data.split_at_mut(1);

        // Check: buffer account should not be finalized.
        let mut metadata = metadata_bits
            .first()
            .and_then(|bits| BufferMetadata::from_bits(*bits))
            .ok_or(ProgramError::InvalidAccountData)?;

        if metadata.is_finalized() {
            msg!("Buffer account is finalized");
            return Err(ProgramError::InvalidAccountData);
        }

        // Deserialize according to command kind.
        // We don't use the value, this step is used just for checking data integrity.
        match metadata.command_kind() {
            CommandKind::ApproveMessage => {
                GatewayExecuteData::<ApproveMessagesVariant>::new(
                    buffer,
                    gateway_root_pda.key,
                    &domain_separator,
                )
                .map_err(|error| {
                    msg!("Failed to deserialize execute_data bytes: {}", error);
                    ProgramError::InvalidAccountData
                })?;
            }
            CommandKind::RotateSigner => {
                GatewayExecuteData::<RotateSignersVariant>::new(
                    buffer,
                    gateway_root_pda.key,
                    &domain_separator,
                )
                .map_err(|error| {
                    msg!("Failed to deserialize execute_data bytes: {}", error);
                    ProgramError::InvalidAccountData
                })?;
            }
        }

        // TODO: What's next?

        // Mark buffer as finalized
        metadata.finalize();
        metadata_bits[0] = metadata.bits();

        Ok(())
    }
}

bitflags! {
    /// Represents the options for the `execute_data` PDA account buffer.
    #[derive(Eq, PartialEq)]
    pub struct BufferMetadata: u8 {
        /// Buffer finalization status.
        ///
        /// Finalized     => 1
        /// Not finalized => 0
        const FINALIZED = 1;

        /// The command kind contained in the buffer.
        ///
        /// ApproveMessages => 0
        /// RotateSigners   => 1
        const COMMAND_KIND = 1 << 1;
    }
}

impl BufferMetadata {
    fn new_from_command_kind(command_kind: CommandKind) -> Self {
        match command_kind {
            CommandKind::ApproveMessage => Self::empty(),
            CommandKind::RotateSigner => Self::COMMAND_KIND,
        }
    }

    fn finalize(&mut self) {
        self.insert(Self::FINALIZED);
    }

    /// Returns true if the `FINALIZED` flag is set.
    pub fn is_finalized(&self) -> bool {
        self.contains(Self::FINALIZED)
    }

    /// Returns the internal [`CommandKind`] according to the `COMMAND_KIND`
    /// flag.
    pub fn command_kind(&self) -> CommandKind {
        if self.contains(Self::COMMAND_KIND) {
            CommandKind::RotateSigner
        } else {
            CommandKind::ApproveMessage
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_data_buffer_metadata_flags() {
        // Test all possible combinations of the flags (0 to 3)
        for bits in 0..=3u8 {
            let meta = BufferMetadata::from_bits(bits).unwrap();

            // `Self::is_finalized` should return `true` if the `FINALIZED` flag is set, and
            // `false` otherwise.
            assert_eq!(
                meta.is_finalized(),
                meta.contains(BufferMetadata::FINALIZED),
                "Method `is_finalized` failed for bits {:02b}",
                bits
            );

            // `Self::command_kind()` should return `CommandKind::ApproveMessage` if the
            // `COMMAND_KIND` flag is not set.
            assert_eq!(
                matches!(meta.command_kind(), CommandKind::ApproveMessage),
                !meta.contains(BufferMetadata::COMMAND_KIND),
                "Invalid output for `command_kind` method for bits {:02b}",
                bits
            );

            // `Self::command_kind()` should return `CommandKind::RotateSigner` if the
            // `COMMAND_KIND` flag is set.
            assert_eq!(
                matches!(meta.command_kind(), CommandKind::RotateSigner),
                meta.contains(BufferMetadata::COMMAND_KIND),
                "Invalid output for `command_kind` method for bits {:02b}",
                bits
            );
        }
    }
}
