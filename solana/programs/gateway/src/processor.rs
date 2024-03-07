//! Program state processor.

use std::borrow::Cow;

use axelar_message_primitives::{
    AxelarMessageParams, CommandId, DataPayloadHash, DestinationProgramId, SourceAddress,
    SourceChain,
};
use borsh::from_slice;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::keccak::hash;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction, system_program};

use crate::accounts::transfer_operatorship::TransferOperatorshipAccount;
use crate::accounts::{GatewayApprovedMessage, GatewayConfig, GatewayExecuteData};
use crate::error::GatewayError;
use crate::events::{
    emit_call_contract_event, emit_message_approved_event, emit_operatorship_transferred_event,
};
use crate::instructions::GatewayInstruction;
use crate::types::execute_data_decoder::DecodedMessage;
use crate::{check_program_account, get_gateway_root_config_pda};

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = from_slice::<GatewayInstruction>(input)?;
        check_program_account(*program_id)?;

        match instruction {
            GatewayInstruction::Execute {} => {
                msg!("Instruction: Execute");
                Self::execute(program_id, accounts)
            }
            GatewayInstruction::CallContract {
                destination_chain,
                destination_contract_address: destination_address,
                payload,
            } => {
                msg!("Instruction: Call Contract");
                let accounts_iter = &mut accounts.iter();
                let sender = next_account_info(accounts_iter)?;
                let payload_hash = hash(&payload).to_bytes();

                assert!(sender.is_signer);

                emit_call_contract_event(
                    *sender.key,
                    destination_chain,
                    destination_address,
                    payload,
                    payload_hash,
                )?;
                Ok(())
            }
            GatewayInstruction::InitializeConfig { config } => {
                msg!("Instruction: Initialize Config");
                Self::initialize_config(accounts, &config)
            }
            GatewayInstruction::InitializeExecuteData { execute_data } => {
                msg!("Instruction: Initialize Execute Data");
                Self::initialize_execute_data(accounts, &execute_data)
            }
            GatewayInstruction::TransferOperatorship {} => {
                msg!("Instruction: TransferOperatorship");
                Self::transfer_operatorship(program_id, accounts)
            }
            GatewayInstruction::InitializeMessage {
                command_id: message_id,
                source_chain,
                source_address,
                payload_hash,
                destination_program,
            } => {
                msg!("Instruction: Initialize Approved Message");
                Self::initialize_approved_message(
                    program_id,
                    accounts,
                    message_id,
                    source_chain,
                    source_address,
                    payload_hash,
                    destination_program,
                )
            }
            GatewayInstruction::InitializeTransferOperatorship {
                operators_and_weights,
                threshold,
            } => {
                msg!("Instruction: Initialize TransferOperatorship");
                Self::initialize_transfer_operatorship(accounts, operators_and_weights, threshold)
            }
            GatewayInstruction::ValidateContractCall {
                destination_program,
                command_id: message_id,
                payload_hash,
                source_address,
                source_chain,
            } => {
                msg!("Instruction: Validate Contract Call");
                Self::validate_contract_call(
                    program_id,
                    message_id,
                    source_chain,
                    source_address,
                    payload_hash,
                    destination_program,
                    accounts,
                )
            }
        }
    }

    fn execute(program_id: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        let mut accounts = accounts.iter();
        let gateway_root_pda = next_account_info(&mut accounts)?;
        let execute_data_account = next_account_info(&mut accounts)?;
        let message_accounts: Vec<_> = accounts.collect();
        // Phase 1: Account validation

        // Check: Config account uses the canonical bump.
        let (canonical_pda, _canonical_bump) = get_gateway_root_config_pda();
        if *gateway_root_pda.key != canonical_pda {
            return Err(GatewayError::InvalidConfigAccount)?;
        }

        // Check: Config account is owned by the Gateway program.
        if *gateway_root_pda.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check: Config account is read only.
        if gateway_root_pda.is_writable {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Check: execute_data account is owned by the Gateway program.
        if *execute_data_account.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check: execute_data account was initialized.
        let execute_data: GatewayExecuteData =
            borsh::from_slice(*execute_data_account.data.borrow())?;

        // Check: at least one message account.
        if message_accounts.is_empty() {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Phase 2: Deserialization & Proof Validation

        let Ok((proof, command_batch)) = execute_data.decode() else {
            return Err(GatewayError::MalformedProof)?;
        };

        // Check: proof is valid, internally.
        proof.validate(&command_batch.hash)?;

        // Unpack Gateway configuration data.
        let gateway_config: GatewayConfig = {
            let gateway_account_bytes = gateway_root_pda.try_borrow_mut_data()?;
            borsh::de::from_slice(&gateway_account_bytes)?
        };

        // Check: proof operators are known.
        gateway_config.validate_proof_operators(&proof.operators)?;

        // Phase 3: Update approved message accounts

        // Check approved message account initial premises post-validation so we iterate
        // on them only once.
        // TODO: Pairwise iterate over message accounts and validated commands from the
        // command batch.
        let mut last_visited_message_index = 0;
        for (&message_account, approved_command) in
            message_accounts.iter().zip(command_batch.commands.iter())
        {
            last_visited_message_index += 1;

            // Check: Current message account represents the current approved command.
            let (expected_pda, _bump, _hash) =
                GatewayApprovedMessage::pda(gateway_root_pda.key, &approved_command.into());
            if expected_pda != *message_account.key {
                return Err(ProgramError::InvalidSeeds);
            }

            // Check:: All messages must be "Approved".
            let mut approved_message =
                message_account.check_initialized_pda::<GatewayApprovedMessage>(program_id)?;

            if !approved_message.is_pending() {
                // TODO: use a more descriptive GatewayError variant here.
                return Err(ProgramError::AccountAlreadyInitialized);
            }

            // Success: update account message state to "Approved".
            // The message by default is in "approved" state.
            // https://github.com/axelarnetwork/axelar-cgp-solidity/blob/968c8964061f594c80dd111887edb93c5069e51e/contracts/AxelarGateway.sol#L509
            approved_message.set_approved();
            let mut data = message_account.try_borrow_mut_data()?;
            approved_message.pack_into_slice(&mut data);

            // Emit an event signaling message approval.
            let DecodedMessage {
                id,
                source_chain,
                source_address,
                destination_address,
                payload_hash,
                ..
            } = &approved_command.message;
            emit_message_approved_event(
                *id,
                source_chain.clone(),
                source_address.clone(),
                *destination_address,
                *payload_hash,
            )?;
        }

        // Check: all messages were visited
        if last_visited_message_index != message_accounts.len()
            || last_visited_message_index != command_batch.commands.len()
        {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        Ok(())
    }

    fn initialize_config(
        accounts: &[AccountInfo],
        gateway_config: &GatewayConfig,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Gateway Config account uses the canonical bump.
        let (canonical_pda, canonical_bump) = crate::get_gateway_root_config_pda();
        if *gateway_root_pda.key != canonical_pda {
            return Err(GatewayError::InvalidConfigAccount.into());
        }

        init_pda(
            payer,
            gateway_root_pda,
            &[&[canonical_bump]],
            gateway_config,
        )
    }

    fn initialize_execute_data(
        accounts: &[AccountInfo<'_>],
        execute_data: &GatewayExecuteData,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let _gateway_root_pda = next_account_info(accounts_iter)?;
        let execute_data_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Execute Data account uses the canonical bump.
        let (canonical_pda, bump, seeds) = execute_data.pda();
        if *execute_data_account.key != canonical_pda {
            return Err(GatewayError::InvalidExecuteDataAccount.into());
        }
        init_pda(
            payer,
            execute_data_account,
            &[seeds.as_ref(), &[bump]],
            execute_data,
        )
    }

    fn initialize_approved_message(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        message_id: [u8; 32],
        source_chain: String,
        source_address: Vec<u8>,
        payload_hash: [u8; 32],
        destination_program: DestinationProgramId,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let approved_message_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // TODO validate gateway root pda

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Approved message account uses the canonical bump.
        let (canonical_pda, bump, seeds) = GatewayApprovedMessage::pda(
            gateway_root_pda.key,
            &AxelarMessageParams {
                command_id: CommandId(Cow::Owned(message_id)),
                source_chain: SourceChain(Cow::Owned(source_chain)),
                source_address: SourceAddress(&source_address),
                destination_program,
                payload_hash: DataPayloadHash(Cow::Borrowed(&payload_hash)),
            },
        );

        approved_message_pda.check_uninitialized_pda()?;
        if *approved_message_pda.key != canonical_pda {
            return Err(GatewayError::InvalidApprovedMessageAccount.into());
        }

        program_utils::init_pda(
            payer,
            approved_message_pda,
            program_id,
            system_account,
            GatewayApprovedMessage::pending(bump),
            &[seeds.as_ref(), &[bump]],
        )
    }

    fn transfer_operatorship(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<(), ProgramError> {
        // Extract required accounts.
        let accounts_iter = &mut accounts.iter();
        let payer_account = next_account_info(accounts_iter)?;
        let new_operators_account = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: Config account is the canonical PDA.
        let (expected_pda_info, _bump) = crate::get_gateway_root_config_pda();
        helper::compare_address(gateway_root_pda, expected_pda_info)?;

        // Check: Config account is owned by the Gateway program.
        if *gateway_root_pda.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check: New operators account is owned by the Gateway program.
        if *new_operators_account.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Unpack the data from the new operators account.
        let new_operators_bytes: &[u8] = &new_operators_account.data.borrow();
        let new_operators =
            borsh::de::from_slice::<TransferOperatorshipAccount>(new_operators_bytes)?;

        // Check: New operators account is the expected PDA.
        let (expected_new_operators_pda, _bump) = new_operators.pda();
        helper::compare_address(new_operators_account, expected_new_operators_pda)?;

        // Check: new operator data is valid.
        new_operators.validate().map_err(GatewayError::from)?;

        // Hash the new operator set.
        let new_operators_hash = new_operators.hash();

        // Unpack Gateway configuration data.
        let mut config: GatewayConfig = {
            let state_bytes_ref = gateway_root_pda.try_borrow_mut_data()?;
            borsh::de::from_slice(&state_bytes_ref)?
        };

        // Update epoch and operators.
        config
            .operators_and_epochs
            .update(new_operators_hash)
            .map_err(GatewayError::from)?;

        // Resize and refund state account space.
        config.reallocate(gateway_root_pda, payer_account, system_account)?;

        // Write the packed data back to the state account.
        let serialized_state = borsh::to_vec(&config)?;
        let mut state_data_ref = gateway_root_pda.try_borrow_mut_data()?;
        state_data_ref[..serialized_state.len()].copy_from_slice(&serialized_state);

        // Emit an event to signal the successful operatorship transfer
        emit_operatorship_transferred_event(*new_operators_account.key)?;
        Ok(())
    }

    fn initialize_transfer_operatorship(
        accounts: &[AccountInfo],
        operators_and_weights: Vec<(crate::types::address::Address, crate::types::u256::U256)>,
        threshold: crate::types::u256::U256,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let transfer_operatorship_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Transfer operatorship account uses the canonical bump.
        let transfer_operatorship =
            TransferOperatorshipAccount::new(operators_and_weights, threshold);
        let (expected_pda, bump, seeds) = transfer_operatorship.pda_with_seeds();

        if *transfer_operatorship_account.key != expected_pda {
            return Err(GatewayError::InvalidAccountAddress.into());
        }

        init_pda(
            payer,
            transfer_operatorship_account,
            &[seeds.as_ref(), &[bump]],
            &transfer_operatorship,
        )
    }

    fn validate_contract_call(
        program_id: &Pubkey,
        command_id: [u8; 32],
        source_chain: String,
        source_address: Vec<u8>,
        payload_hash: [u8; 32],
        destination_pubkey: DestinationProgramId,
        accounts: &[AccountInfo],
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let approved_message_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let caller = next_account_info(accounts_iter)?;

        let mut approved_message =
            approved_message_pda.check_initialized_pda::<GatewayApprovedMessage>(program_id)?;

        let message_id = CommandId(Cow::Owned(command_id));
        let seed_hash = GatewayApprovedMessage::calculate_seed_hash(
            gateway_root_pda.key,
            &AxelarMessageParams {
                command_id: message_id.clone(),
                source_chain: SourceChain(Cow::Owned(source_chain)),
                source_address: SourceAddress(&source_address),
                destination_program: destination_pubkey,
                payload_hash: DataPayloadHash(Cow::Borrowed(&payload_hash)),
            },
        );
        let approved_message_derived_pda = Pubkey::create_program_address(
            &[seed_hash.as_ref(), &[approved_message.bump]],
            program_id,
        )?;
        if *approved_message_pda.key != approved_message_derived_pda {
            return Err(GatewayError::InvalidAccountAddress.into());
        }

        // Action
        approved_message.verify_caller(&message_id, &destination_pubkey, caller)?;

        // Store the data back to the account.
        let mut data = approved_message_pda.try_borrow_mut_data()?;
        approved_message.pack_into_slice(&mut data);

        Ok(())
    }
}

/// Initialize a Gateway PDA
fn init_pda<'a, 'b, T: borsh::BorshSerialize>(
    payer: &'a AccountInfo<'b>,
    new_account_pda: &'a AccountInfo<'b>,
    seeds: &[&[u8]],
    data: &T,
) -> Result<(), ProgramError> {
    let serialized_data = borsh::to_vec(data)?;
    let space = serialized_data.len();
    let rent_sysvar = Rent::get()?;
    let rent = rent_sysvar.minimum_balance(space);

    assert!(payer.is_signer);
    assert!(payer.is_writable);
    // Note that `new_account_pda` is not a signer yet.
    // This program will sign for it via `invoke_signed`.
    assert!(!new_account_pda.is_signer);
    assert!(new_account_pda.is_writable);
    assert_eq!(new_account_pda.owner, &system_program::ID);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            new_account_pda.key,
            rent,
            space
                .try_into()
                .map_err(|_| ProgramError::ArithmeticOverflow)?,
            &crate::ID,
        ),
        &[payer.clone(), new_account_pda.clone()],
        &[seeds],
    )?;
    let mut account_data = new_account_pda.try_borrow_mut_data()?;
    account_data[..space].copy_from_slice(&serialized_data);
    Ok(())
}

mod helper {
    use solana_program::account_info::AccountInfo;
    use solana_program::pubkey::Pubkey;

    use crate::error::GatewayError;

    /// Compares the account address with the expected address.
    pub(super) fn compare_address(
        pda_info: &AccountInfo<'_>,
        expected_pda_info: Pubkey,
    ) -> Result<(), GatewayError> {
        if pda_info.key != &expected_pda_info {
            Err(GatewayError::IncorrectAccountAddr)
        } else {
            Ok(())
        }
    }
}
