use std::borrow::Cow;

use axelar_message_primitives::command::RotateSignersCommand;
use axelar_message_primitives::Address;
use axelar_rkyv_encoding::types::ArchivedVerifierSet;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;

use super::Processor;
use crate::axelar_auth_weighted::SignerSetMetadata;
use crate::commands::ArchivedCommand;
use crate::events::GatewayEvent;
use crate::state::execute_data::{ArchivedGatewayExecuteData, RotateSignersVariant};
use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::state::{GatewayApprovedCommand, GatewayConfig};
use crate::{assert_valid_verifier_set_tracker_pda, hasher_impl, seed_prefixes};

impl Processor {
    /// Rotate the weighted signers, signed off by the latest Axelar signers.
    /// The minimum rotation delay is enforced by default, unless the caller is
    /// the gateway operator.
    ///
    /// The gateway operator allows recovery in case of an incorrect/malicious
    /// rotation, while still requiring a valid proof from a recent signer set.
    ///
    /// Rotation to duplicate signers is rejected.
    ///
    /// reference implementation: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/9dae93af0b799e536005951ddc36284132813579/contracts/gateway/AxelarAmplifierGateway.sol#L94
    pub fn process_rotate_signers(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
    ) -> ProgramResult {
        let mut accounts_iter = accounts.iter();
        let gateway_root_pda = next_account_info(&mut accounts_iter)?;
        let gateway_approve_messages_execute_data_pda = next_account_info(&mut accounts_iter)?;
        let message_account = next_account_info(&mut accounts_iter)?;
        let signer_verifier_set = next_account_info(&mut accounts_iter)?;
        let new_empty_verifier_set = next_account_info(&mut accounts_iter)?;
        let payer = next_account_info(&mut accounts_iter)?;
        let system_account = next_account_info(&mut accounts_iter)?;
        let operator = next_account_info(&mut accounts_iter);

        // Check: Config account uses the canonical bump.
        // Unpack Gateway configuration data.
        let mut gateway_config =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        // Validate teh PDAs of the verifier sets
        let signer_verifier_set =
            match signer_verifier_set.check_initialized_pda::<VerifierSetTracker>(program_id) {
                Ok(set) => set,
                Err(err) => {
                    msg!("Invalid VerifierSetTracker PDA");
                    return Err(err);
                }
            };
        new_empty_verifier_set.check_uninitialized_pda()?;

        // we always enforce the delay unless unless the operator has been provided and
        // its also the Gateway opreator
        // refence: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/c290c7337fd447ecbb7426e52ac381175e33f602/contracts/gateway/AxelarAmplifierGateway.sol#L98-L101
        let enforce_rotation_delay = operator.map_or(true, |operator| {
            let operator_matches = *operator.key == gateway_config.operator;
            let operator_is_sigener = operator.is_signer;
            // if the operator matches and is also the signer - disable rotation delay
            !(operator_matches && operator_is_sigener)
        });

        gateway_approve_messages_execute_data_pda
            .check_initialized_pda_without_deserialization(program_id)?;

        let borrowed_account_data = gateway_approve_messages_execute_data_pda.data.borrow();

        let Ok(execute_data) =
            ArchivedGatewayExecuteData::<RotateSignersVariant>::from_bytes(&borrowed_account_data)
        else {
            return Err(ProgramError::InvalidAccountData);
        };

        let new_verifier_set = &execute_data.data;
        let command = ArchivedCommand::from(new_verifier_set);

        let approved_command_account = message_account
            .as_ref()
            .check_initialized_pda::<GatewayApprovedCommand>(program_id)?
            .command_valid_and_pending(gateway_root_pda.key, &command, message_account)?
            .ok_or_else(|| {
                msg!("Command already executed");
                ProgramError::InvalidArgument
            })?;

        // Check: proof signer set is known.
        let signer_data = gateway_config
            .validate_proof(
                execute_data.payload_hash,
                &execute_data.proof,
                &signer_verifier_set,
            )
            .map_err(|err| {
                msg!("Proof validation failed: {:?}", err);
                ProgramError::InvalidArgument
            })?;

        // Check: proof is signed by latest signers
        if enforce_rotation_delay && !matches!(signer_data, SignerSetMetadata::Latest) {
            msg!("Proof is not signed by the latest signer set");
            return Err(ProgramError::InvalidArgument);
        }

        let current_time: u64 = solana_program::clock::Clock::get()?
            .unix_timestamp
            .try_into()
            .expect("received negative timestamp");
        if enforce_rotation_delay
            && !Self::enough_time_till_next_rotation(current_time, &gateway_config)?
        {
            msg!("Command needs more time before being executed again");
            return Err(ProgramError::InvalidArgument);
        }

        gateway_config.auth_weighted.last_rotation_timestamp = current_time;

        // Save the updated approved message account
        let mut data = message_account.try_borrow_mut_data()?;
        approved_command_account.pack_into_slice(&mut data);

        let new_verifier_set_tracker = match gateway_config.rotate_signers(new_verifier_set) {
            Ok(new_verifier_set_tracker) => new_verifier_set_tracker,
            Err(err) => {
                msg!("Failed to rotate signers {:?}", err);
                return Err(ProgramError::InvalidAccountData);
            }
        };
        assert_valid_verifier_set_tracker_pda(
            &new_verifier_set_tracker,
            new_empty_verifier_set.key,
        );

        program_utils::init_pda(
            payer,
            new_empty_verifier_set,
            program_id,
            system_account,
            new_verifier_set_tracker.clone(),
            &[
                seed_prefixes::VERIFIER_SET_TRACKER_SEED,
                new_verifier_set_tracker.verifier_set_hash.as_slice(),
                &[new_verifier_set_tracker.bump],
            ],
        )?;

        // Emit event if the signers were rotated
        emit_signers_rotated_event(new_verifier_set)?;

        // Store the gateway data back to the account.
        let mut data = gateway_root_pda.try_borrow_mut_data()?;
        gateway_config.pack_into_slice(&mut data);

        Ok(())
    }

    fn enough_time_till_next_rotation(
        current_time: u64,
        config: &GatewayConfig,
    ) -> Result<bool, ProgramError> {
        let secs_since_last_rotation = current_time
            .checked_sub(config.auth_weighted.last_rotation_timestamp)
            .expect("Current time minus rotate signers last successful operation time should not underflow");
        Ok(secs_since_last_rotation >= config.auth_weighted.minimum_rotation_delay)
    }
}

/// FIXME: Temporary workaround to emit a 'SignersRotated' event without
/// breaking the public API (currently used by AMPD and the Relayer). Once we
/// use 'axelar-rkyv-encoding' types across all APIs this can be revisited and
/// adjusted.
fn emit_signers_rotated_event(verifier_set: &ArchivedVerifierSet) -> Result<(), ProgramError> {
    let size = verifier_set.size();

    let mut signer_set = Vec::with_capacity(size);
    let mut weights = Vec::with_capacity(size);

    for (pubkey, weight) in verifier_set.signers() {
        let pubkey = pubkey.to_bytes();
        let weight: u128 = weight.into();
        let Ok(address) = Address::try_from(pubkey.as_slice()) else {
            msg!("Invalid public key length: {}", pubkey.len());
            return Err(ProgramError::InvalidArgument);
        };
        signer_set.push(address);
        weights.push(weight);
    }

    let quorum: u128 = verifier_set.quorum().into();

    let rotate_signers_command = RotateSignersCommand {
        command_id: verifier_set.hash(hasher_impl()),
        destination_chain: 0, // XXX: the chain ID is not relevant for rotating signers.
        signer_set,
        weights,
        quorum,
    };

    GatewayEvent::SignersRotated(Cow::Owned(rotate_signers_command)).emit()
}
