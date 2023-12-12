//! Proof validation logic

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::secp256k1_recover;

use super::AuthWeightedInstruction;
use crate::error::AuthWeightedError;
use crate::types::account::state::AuthWeightedStateAccount;
use crate::types::account::validate_proof::ValidateProofAccount;
use crate::types::proof::Proof;
use crate::types::u256::U256;
use crate::{check_program_account, id};

/// Creates a validate proof instruction.
/// Thats purely for testing purposes.
pub fn build_validate_proof_ix(
    payer: &Pubkey,
    params: &Pubkey,
    state: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*params, false),
        AccountMeta::new_readonly(*state, false),
    ];

    let data = AuthWeightedInstruction::ValidateProof.pack();

    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}

/// This function takes messageHash and proof data and reverts if proof is
/// invalid Returns [true] if provided operators are the current ones.
pub fn validate_proof_ix(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<bool, ProgramError> {
    // Number of recent operator sets to be tracked.
    const OLD_KEY_RETENTION: u8 = 16;

    check_program_account(program_id)?;

    let accounts = &mut accounts.iter();

    // A payer isn't used here, but to get further accounts we have to get it first.
    let _ = next_account_info(accounts)?;

    // Account with message hash and proof.
    let params = next_account_info(accounts)?;

    // Account with program state.
    let state = next_account_info(accounts)?;

    // Params account data.
    let params_data: &[u8] = &params.data.borrow();
    let params_data = match ValidateProofAccount::unpack(params_data) {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };

    // State account data.
    let state_data: &[u8] = &state.data.borrow();
    let state_data = match AuthWeightedStateAccount::unpack(state_data) {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };

    let operators_hash = params_data.proof.get_operators_hash();
    let message_hash = params_data.message_hash;

    let operators_epoch = match state_data.epoch_for_hash.get(&operators_hash) {
        Some(epoch) => epoch,
        None => return Err(AuthWeightedError::EpochForHashNotFound.into()),
    };

    let epoch = &state_data.current_epoch;

    if operators_epoch == &U256::from(0) || epoch - operators_epoch >= U256::from(OLD_KEY_RETENTION)
    {
        return Err(AuthWeightedError::InvalidOperators.into());
    };

    match validate_signatures(&message_hash, &params_data.proof) {
        Ok(_) => {}
        Err(e) => return Err(e.into()),
    };

    Ok(operators_epoch == epoch)
}

/// Perform signatures validation with engagement of secp256k1 recovery
/// similarly to ethereum ECDSA recovery.
fn validate_signatures(message_hash: &[u8; 32], proof: &Proof) -> Result<(), AuthWeightedError> {
    let operators_len = proof.operators.addresses_len();
    let mut operator_index: usize = 0;
    let mut weight = U256::new([0; 32]);

    for v in proof.signatures() {
        let recovery_id = 0; // TODO: check if it has to be switch 0, 1.
        let signer =
            match secp256k1_recover::secp256k1_recover(message_hash, recovery_id, v.signature()) {
                Ok(signer) => signer.to_bytes(),
                Err(e) => match e {
                    secp256k1_recover::Secp256k1RecoverError::InvalidHash => {
                        return Err(AuthWeightedError::Secp256k1RecoveryFailedInvalidHash)
                    }
                    secp256k1_recover::Secp256k1RecoverError::InvalidRecoveryId => {
                        return Err(AuthWeightedError::Secp256k1RecoveryFailedInvalidRecoveryId)
                    }
                    secp256k1_recover::Secp256k1RecoverError::InvalidSignature => {
                        return Err(AuthWeightedError::Secp256k1RecoveryFailedInvalidSignature)
                    }
                },
            };
        // First half of uncompressed key.
        let signer = &signer[..32];

        // Looping through remaining operators to find a match.
        while operator_index < operators_len
            && proof
                .operators
                .address_by_index(operator_index)
                .omit_prefix()
                .ne(signer)
        {
            operator_index += 1;
        }

        // Checking if we are out of operators.
        if operator_index == operators_len {
            return Err(AuthWeightedError::MalformedSigners);
        }

        // Accumulating signatures weight.
        // CHECK: How to get rid of clone.
        weight = weight + proof.operators.weight_by_index(operator_index).to_owned();

        // Weight needs to reach or surpass threshold.
        // CHECK: How to get rid of clone.
        if weight >= *proof.operators.threshold() {
            // msg!("about to return ok");
            return Ok(());
        }

        // Increasing operators index if match was found.
        operator_index += 1;
    }

    Err(AuthWeightedError::LowSignaturesWeight)
}
