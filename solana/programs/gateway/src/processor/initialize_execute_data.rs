use axelar_rkyv_encoding::types::{ArchivedExecuteData, ArchivedPayload};
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
use crate::state::execute_data::ExecuteDataVariant;
use crate::state::execute_data_buffer::{BufferLayout, RESERVED_BUFFER_METADATA_BYTES};
use crate::state::signature_verification::merkle_tree::MerkleProof;
use crate::state::signature_verification::{
    batch_context_from_proof, SignatureNode, SignatureVerification, SignatureVerifier,
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

        // Adjust buffer size to hold extra information.
        let adjusted_buffer_size = buffer_size
            .checked_add(RESERVED_BUFFER_METADATA_BYTES as u64)
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

        // Set the buffer metadata
        let mut data = buffer_account.try_borrow_mut_data()?;
        let mut buffer = BufferLayout::parse(&mut data)?;
        buffer.set_command_kind(command_kind);

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
        let buffer = BufferLayout::parse(&mut data)?;

        // Check: buffer account should not be finalized.
        if buffer.metadata().is_finalized() {
            msg!("Buffer account is finalized");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: Write bounds
        let write_offset = offset.saturating_add(bytes.len());
        if buffer.raw_execute_data.len() < write_offset {
            msg!(
                "Write overflow: {} < {}",
                buffer.raw_execute_data.len(),
                write_offset
            );
            return Err(ProgramError::AccountDataTooSmall);
        }

        buffer
            .raw_execute_data
            .get_mut(offset..write_offset)
            .ok_or(ProgramError::AccountDataTooSmall)?
            .copy_from_slice(bytes);

        Ok(())
    }

    /// Handles the
    /// [`crate::instructions::GatewayInstruction::CommitPayloadHash`]
    /// instruction.
    pub fn process_commit_payload_hash(
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
        let mut buffer = BufferLayout::parse(&mut data)?;

        // Check: buffer account should not be finalized.
        {
            let metadata = buffer.metadata();
            if metadata.is_finalized() {
                msg!("Buffer account is already finalized");
                return Err(ProgramError::InvalidAccountData);
            }

            if metadata.has_payload_hash() {
                msg!("Payload hash have already been calculated");
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // Deserialize / Unarchive
        let archived_execute_data = ArchivedExecuteData::from_bytes(buffer.raw_execute_data)
            .map_err(|error| {
                msg!("Failed to deserialize execute_data: {}", error);
                ProgramError::InvalidAccountData
            })?;

        // Check: Buffer metadata `COMMAND_KIND` matches buffer content.
        if !matches!(
            (
                buffer.metadata().command_kind(),
                &archived_execute_data.payload
            ),
            (CommandKind::ApproveMessage, ArchivedPayload::Messages(_))
                | (CommandKind::RotateSigner, ArchivedPayload::VerifierSet(_))
        ) {
            msg!("Buffer metadata COMMAND_KIND doesn't match its contents");
            return Err(GatewayError::InvalidExecuteDataAccount)?;
        }

        // Hash the payload and persist into the buffer account.
        let payload_hash =
            archived_execute_data.internal_payload_hash(&domain_separator, crate::hasher_impl());
        buffer.payload_hash.copy_from_slice(payload_hash.as_slice());

        buffer.commit_payload_hash(&payload_hash)
    }

    /// Handles the
    /// [`crate::instructions::GatewayInstruction::InitializeSignatureVerification`]
    /// instruction.
    pub fn process_initialize_signature_verification(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        merkle_root: &[u8; 32],
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
        let mut buffer = BufferLayout::parse(&mut data)?;

        // Check: buffer account should have its payload hash calculated.
        if !buffer.metadata().has_payload_hash() {
            msg!("Payload hash has not been calculated yet");
            return Err(ProgramError::InvalidAccountData);
        }

        // Deserialize / Unarchive
        let archived_execute_data = ArchivedExecuteData::from_bytes(buffer.raw_execute_data)
            .map_err(|error| {
                msg!("Failed to deserialize execute_data: {}", error);
                ProgramError::InvalidAccountData
            })?;

        let batch_context = batch_context_from_proof(
            *gateway_root_pda.key,
            domain_separator,
            &archived_execute_data.proof,
            *buffer.payload_hash,
        )?;
        buffer.initialize_signature_verification(merkle_root, batch_context);

        Ok(())
    }

    /// Handles the
    /// [`crate::instructions::GatewayInstruction::InitializeSignatureVerification`]
    /// instruction.
    pub fn process_verify_signature(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        signature_bytes: &[u8],
        public_key_bytes: &[u8],
        signer_weight: u128,
        signer_index: u8,
        signature_merkle_proof: &[u8],
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
        let mut buffer = BufferLayout::parse(&mut data)?;

        // Check: buffer account should have its payload hash calculated.
        if !buffer.metadata().has_payload_hash() {
            msg!("Payload hash has not been calculated yet");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: buffer account should have a non-empty signature merkle root
        let mut signature_verification =
            SignatureVerification::deserialize(buffer.signature_verification);
        if signature_verification.root().iter().all(|x| x == &0) {
            msg!("Merkle root has not been initialized");
            return Err(ProgramError::InvalidAccountData);
        }

        // Deserialize / Unarchive
        let archived_execute_data = ArchivedExecuteData::from_bytes(buffer.raw_execute_data)
            .map_err(|error| {
                msg!("Failed to deserialize execute_data: {}", error);
                ProgramError::InvalidAccountData
            })?;

        let batch_context = batch_context_from_proof(
            *gateway_root_pda.key,
            domain_separator,
            &archived_execute_data.proof,
            *buffer.payload_hash,
        )?;

        // Build the signature leaf node
        let signature_node = SignatureNode::new(
            signature_bytes,
            public_key_bytes,
            signer_weight,
            signer_index,
            &batch_context,
        );

        // Deserialize the signature merkle proof
        let merkle_proof = MerkleProof::from_bytes(signature_merkle_proof).map_err(|_| {
            msg!("Signature does not belong to this verification session");
            ProgramError::InvalidArgument
        })?;

        // Verify if the signature leaf node belongs to this  verification session
        signature_verification.verify_signature(
            &signature_node,
            merkle_proof,
            GatewaySignatureVerifier {},
        );

        // Update the signature verification session
        buffer.update_signature_verification(signature_verification);

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
        let buffer_account = next_account_info(accounts_iter)?;
        assert!(buffer_account.is_writable);
        assert_eq!(buffer_account.owner, program_id);

        let mut data = buffer_account.try_borrow_mut_data()?;
        let mut buffer = BufferLayout::parse(&mut data)?;

        // Check: buffer account should not be finalized.
        if buffer.metadata().is_finalized() {
            msg!("Buffer account is already finalized");
            return Err(ProgramError::InvalidAccountData);
        }

        buffer.finalize()
    }
}

struct GatewaySignatureVerifier {}
impl SignatureVerifier<&[u8], &[u8]> for GatewaySignatureVerifier {
    /// TODO: Implement this
    fn verify_signature(
        &self,
        _signature: &&[u8],
        _public_key: &&[u8],
        _message: &[u8; 32],
    ) -> Option<u128> {
        // WARN: This will always verify the signature without looking at the inputs
        // TODO: implement this
        Some(u128::MAX)
    }
}
