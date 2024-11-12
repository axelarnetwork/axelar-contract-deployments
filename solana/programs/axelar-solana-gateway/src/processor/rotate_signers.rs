use axelar_message_primitives::U256;
use axelar_rkyv_encoding::hasher::merkle_tree::{Hasher, SolanaSyscallHasher};
use axelar_rkyv_encoding::types::VerifierSet;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;
use solana_program::{keccak, msg};

use super::Processor;
use crate::events::{GatewayEvent, RotateSignersEvent};
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::state::GatewayConfig;
use crate::{assert_valid_verifier_set_tracker_pda, seed_prefixes};

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
        new_verifier_set_merkle_root: [u8; 32],
        new_verifier_set_bump: u8,
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let verifier_set_tracker_account = next_account_info(accounts_iter)?;
        let new_empty_verifier_set = next_account_info(accounts_iter)?;
        let payer = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;
        let operator = next_account_info(accounts_iter);

        // Check: Gateway Root PDA is initialized.
        let mut gateway_config =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        let mut data = verification_session_account.try_borrow_mut_data()?;
        let data_bytes: &mut [u8; SignatureVerificationSessionData::LEN] =
            (*data).try_into().map_err(|_err| {
                msg!("session account data is corrupt");
                ProgramError::InvalidAccountData
            })?;
        let session = bytemuck::cast_mut::<_, SignatureVerificationSessionData>(data_bytes);
        if !session.signature_verification.is_valid() {
            msg!("signing session is not complete");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: new verifier set merkle root can be transformed into the payload hash
        let verifier_set_leaf_node = keccak::hashv(&[
            VerifierSet::HASH_PREFIX,
            new_verifier_set_merkle_root.as_ref(),
        ])
        .0;
        let expected_payload_merkle_root =
            SolanaSyscallHasher::concat_and_hash(&verifier_set_leaf_node, None);

        // Check: Verification PDA can be derived from seeds stored into the account
        // data itself.
        {
            let expected_pda = crate::create_signature_verification_pda(
                gateway_root_pda.key,
                &expected_payload_merkle_root,
                session.bump,
            )?;
            if expected_pda != *verification_session_account.key {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        // Obtain the active verifier set tracker.
        let verifier_set_tracker = verifier_set_tracker_account
            .check_initialized_pda::<VerifierSetTracker>(program_id)
            .map_err(|error| {
                msg!("Invalid VerifierSetTracker PDA");
                error
            })?;

        // Check: we got the expected verifier set
        if verifier_set_tracker.verifier_set_hash
            != session.signature_verification.signing_verifier_set_hash
        {
            msg!("Provided verifier set tracker PDA does not match the verifier set that signed the signing sesseion");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: Verifier set isn't expired
        let is_epoch_valid = gateway_config
            .is_epoch_valid(verifier_set_tracker.epoch)
            .map_err(|err| {
                msg!("AuthWeightedError: {}", err);
                ProgramError::InvalidInstructionData
            })?;
        if !is_epoch_valid {
            msg!("Expired VerifierSetTracker PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        // Check: new new verifier set PDA must be uninitialised
        new_empty_verifier_set.check_uninitialized_pda()?;

        // we always enforce the delay unless unless the operator has been provided and
        // its also the Gateway opreator
        // reference: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/c290c7337fd447ecbb7426e52ac381175e33f602/contracts/gateway/AxelarAmplifierGateway.sol#L98-L101
        let enforce_rotation_delay = operator.map_or(true, |operator| {
            let operator_matches = *operator.key == gateway_config.operator;
            let operator_is_sigener = operator.is_signer;
            // if the operator matches and is also the signer - disable rotation delay
            !(operator_matches && operator_is_sigener)
        });
        let is_latest = gateway_config.current_epoch == verifier_set_tracker.epoch;
        // Check: proof is signed by latest signers
        if enforce_rotation_delay && !is_latest {
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

        // Rotate the signers
        gateway_config.current_epoch = gateway_config
            .current_epoch
            .checked_add(U256::ONE)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let new_verifier_set_tracker = VerifierSetTracker {
            bump: new_verifier_set_bump,
            epoch: gateway_config.current_epoch,
            verifier_set_hash: new_verifier_set_merkle_root,
        };

        assert_valid_verifier_set_tracker_pda(
            &new_verifier_set_tracker,
            new_empty_verifier_set.key,
        )?;

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
        GatewayEvent::SignersRotated(RotateSignersEvent {
            new_epoch: new_verifier_set_tracker.epoch,
            new_signers_hash: new_verifier_set_tracker.verifier_set_hash,
        })
        .emit()?;

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
