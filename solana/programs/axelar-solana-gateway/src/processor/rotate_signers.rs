use std::convert::TryInto;
use std::mem::size_of;

use axelar_message_primitives::U256;
use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;

use super::event_utils::{read_array, EventParseError};
use super::Processor;
use crate::error::GatewayError;
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::state::{BytemuckedPda, GatewayConfig};
use crate::{
    assert_valid_gateway_root_pda, assert_valid_signature_verification_pda,
    assert_valid_verifier_set_tracker_pda, event_prefixes, get_verifier_set_tracker_pda,
    seed_prefixes,
};

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
    pub fn process_rotate_verifier_set(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        new_verifier_set_merkle_root: [u8; 32],
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
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;
        let mut data = gateway_root_pda.try_borrow_mut_data()?;
        let gateway_config = GatewayConfig::read_mut(&mut data)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: Verification session PDA is initialized.
        verification_session_account.check_initialized_pda_without_deserialization(program_id)?;
        let mut data = verification_session_account.try_borrow_mut_data()?;
        let session = SignatureVerificationSessionData::read_mut(&mut data)?;

        // New verifier set merkle root can be transformed into the payload hash
        let payload_merkle_root =
            axelar_solana_encoding::types::verifier_set::construct_payload_hash::<
                SolanaSyscallHasher,
            >(
                new_verifier_set_merkle_root,
                session.signature_verification.signing_verifier_set_hash,
            );

        // Check: Verification PDA can be derived from seeds stored into the account
        // data itself.
        assert_valid_signature_verification_pda(
            gateway_root_pda.key,
            &payload_merkle_root,
            session.bump,
            verification_session_account.key,
        )?;

        if !session.signature_verification.is_valid() {
            return Err(GatewayError::SigningSessionNotValid.into());
        }

        // Check: Active verifier set tracker PDA is initialized.
        verifier_set_tracker_account.check_initialized_pda_without_deserialization(program_id)?;
        let data = verifier_set_tracker_account.try_borrow_data()?;
        let verifier_set_tracker = VerifierSetTracker::read(&data)?;
        assert_valid_verifier_set_tracker_pda(
            verifier_set_tracker,
            verifier_set_tracker_account.key,
        )?;

        // Check: we got the expected verifier set
        if verifier_set_tracker.verifier_set_hash
            != session.signature_verification.signing_verifier_set_hash
        {
            return Err(GatewayError::InvalidVerifierSetTrackerProvided.into());
        }

        // Check: Current verifier set isn't expired
        gateway_config.assert_valid_epoch(verifier_set_tracker.epoch)?;

        // Check: new new verifier set PDA must be uninitialised
        new_empty_verifier_set
            .check_uninitialized_pda()
            .map_err(|_err| GatewayError::VerifierSetTrackerAlreadyInitialised)?;

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
        // Check: proof is signed by latest verifiers
        if enforce_rotation_delay && !is_latest {
            return Err(GatewayError::ProofNotSignedByLatestVerifierSet.into());
        }

        let current_time: u64 = solana_program::clock::Clock::get()?
            .unix_timestamp
            .try_into()
            .expect("received negative timestamp");
        if enforce_rotation_delay && !enough_time_till_next_rotation(current_time, gateway_config)?
        {
            return Err(GatewayError::RotationCooldownNotDone.into());
        }

        gateway_config.last_rotation_timestamp = current_time;

        // Rotate the signers
        gateway_config.current_epoch = gateway_config
            .current_epoch
            .checked_add(U256::ONE)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Initialize the tracker account
        let (_, new_verifier_set_bump) = get_verifier_set_tracker_pda(new_verifier_set_merkle_root);
        program_utils::init_pda_raw(
            payer,
            new_empty_verifier_set,
            program_id,
            system_account,
            size_of::<VerifierSetTracker>() as u64,
            &[
                seed_prefixes::VERIFIER_SET_TRACKER_SEED,
                new_verifier_set_merkle_root.as_slice(),
                &[new_verifier_set_bump],
            ],
        )?;

        // store account data
        let mut data = new_empty_verifier_set.try_borrow_mut_data()?;
        let new_verifier_set_tracker = VerifierSetTracker::read_mut(&mut data)?;
        *new_verifier_set_tracker = VerifierSetTracker {
            bump: new_verifier_set_bump,
            _padding: [0; 7],
            epoch: gateway_config.current_epoch,
            verifier_set_hash: new_verifier_set_merkle_root,
        };

        // check that everything has been derived correctly
        assert_valid_verifier_set_tracker_pda(
            new_verifier_set_tracker,
            new_empty_verifier_set.key,
        )?;

        // Emit an event
        sol_log_data(&[
            event_prefixes::SIGNERS_ROTATED,
            // u256 as LE [u8; 32]
            &new_verifier_set_tracker.epoch.to_le_bytes(),
            // [u8; 32]
            &new_verifier_set_tracker.verifier_set_hash,
        ]);

        Ok(())
    }
}

fn enough_time_till_next_rotation(
    current_time: u64,
    config: &GatewayConfig,
) -> Result<bool, ProgramError> {
    let secs_since_last_rotation = current_time
        .checked_sub(config.last_rotation_timestamp)
        .expect(
            "Current time minus rotate signers last successful operation time should not underflow",
        );
    Ok(secs_since_last_rotation >= config.minimum_rotation_delay)
}

/// Represents a `SignersRotatedEvent`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VerifierSetRotated {
    /// Epoch of the new verifier set
    pub epoch: U256,
    /// the hash of the new verifier set
    pub verifier_set_hash: [u8; 32],
}

impl VerifierSetRotated {
    /// Constructs a new `SignersRotatedEvent` with the provided data slice.
    pub fn new(mut data: impl Iterator<Item = Vec<u8>>) -> Result<Self, EventParseError> {
        let epoch = read_array::<32>(
            "epoch",
            &data.next().ok_or(EventParseError::MissingData("epoch"))?,
        )?;
        let epoch = U256::from_le_bytes(epoch);

        let verifier_set_hash = read_array::<32>(
            "verifier_set_hash",
            &data
                .next()
                .ok_or(EventParseError::MissingData("verifier_set_hash"))?,
        )?;
        Ok(Self {
            epoch,
            verifier_set_hash,
        })
    }
}
