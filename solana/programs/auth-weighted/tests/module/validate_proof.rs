use std::collections::BTreeMap;

use ::base64::engine::general_purpose;
use anyhow::Result;
use auth_weighted::error::AuthWeightedError;
use auth_weighted::types::account::state::AuthWeightedStateAccount;
use auth_weighted::types::account::validate_proof::ValidateProofAccount;
use auth_weighted::types::address::Address;
use auth_weighted::types::operator::Operators;
use auth_weighted::types::proof::Proof;
use auth_weighted::types::signature::Signature;
use auth_weighted::types::u256::U256;
use base64::Engine;
use solana_program::instruction::InstructionError;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};

use crate::utils::program_test;

// Prepare message_hash and proof.
fn prepare_valid_proof() -> ([u8; 32], Proof) {
    let message_hash: [u8; 32] = {
        let mut array = [0; 32];
        array.copy_from_slice(
            &hex::decode("fa0609efd1dfeedfdcc8ba51520fae2d5176b7621d2560f071e801b0817e1537")
                .expect("decodable input"),
        );

        array
    };

    (
        message_hash,
        Proof::new(
            prepare_valid_operators(vec![U256::from(10)], U256::from(1)),
            vec![Signature::new(
                [
                    0x28, 0x37, 0x86, 0xd8, 0x44, 0xa7, 0xc4, 0xd1, 0xd4, 0x24, 0x83, 0x70, 0x74,
                    0xd0, 0xc8, 0xec, 0x71, 0xbe, 0xcd, 0xcb, 0xa4, 0xdd, 0x42, 0xb5, 0x30, 0x7c,
                    0xb5, 0x43, 0xa0, 0xe2, 0xc8, 0xb8, 0x1c, 0x10, 0xad, 0x54, 0x1d, 0xef, 0xd5,
                    0xce, 0x84, 0xd2, 0xa6, 0x08, 0xfc, 0x45, 0x48, 0x27, 0xd0, 0xb6, 0x5b, 0x48,
                    0x65, 0xc8, 0x19, 0x2a, 0x2e, 0xa1, 0x73, 0x6a, 0x5c, 0x4b, 0x72, 0x02,
                ]
                .into(),
            )],
        ),
    )
}

fn prepare_valid_operators(weights: Vec<U256>, threshold: U256) -> Operators {
    Operators::new(
        vec![Address::new(
            [
                0x03, 0xf5, 0x7d, 0x1a, 0x81, 0x3f, 0xeb, 0xac, 0xcb, 0xe6, 0x42, 0x96, 0x03, 0xf9,
                0xec, 0x57, 0x96, 0x95, 0x11, 0xb7, 0x6c, 0xd6, 0x80, 0x45, 0x2d, 0xba, 0x91, 0xfa,
                0x01, 0xf5, 0x4e, 0x75, 0x6d,
            ]
            .into(),
        )],
        weights,
        threshold,
    )
}

fn prepare_valid_state_account() -> AuthWeightedStateAccount {
    let current_epoch = U256::from(1);

    let mut epoch_for_hash: BTreeMap<[u8; 32], U256> = BTreeMap::new();
    let mut hash_for_epoch: BTreeMap<U256, [u8; 32]> = BTreeMap::new();

    let (_, proof) = prepare_valid_proof();
    let operators_hash = proof.get_operators_hash();

    // Populate.
    epoch_for_hash.insert(operators_hash, current_epoch);
    hash_for_epoch.insert(current_epoch, operators_hash);

    //
    AuthWeightedStateAccount {
        current_epoch,
        epoch_for_hash,
        hash_for_epoch,
    }
}

#[tokio::test]
async fn test_validate_proof_happy_scenario() -> Result<()> {
    // Keys.
    let accounts_owner = Keypair::new();

    let params_account = Keypair::new().pubkey();
    let state_account = Keypair::new().pubkey();

    // Mock data prepare; params account.
    let (message_hash, proof) = prepare_valid_proof();
    let params_account_b64 = general_purpose::STANDARD.encode(
        ValidateProofAccount {
            message_hash,
            proof,
        }
        .pack(),
    );

    // Mock data prepare; program state.
    let state_account_b64 =
        general_purpose::STANDARD.encode(borsh::to_vec(&prepare_valid_state_account())?);

    // Env setup.
    let mut program_test: ProgramTest = program_test();

    // Add account with params; message_hash, proof.
    program_test.add_account_with_base64_data(
        params_account,
        999999,
        accounts_owner.pubkey(),
        &params_account_b64,
    );

    // Add account with state; message_hash, proof.
    program_test.add_account_with_base64_data(
        state_account,
        999999,
        accounts_owner.pubkey(),
        &state_account_b64,
    );

    // Kick.
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Prepare instruction; for tests only.
    let instruction = auth_weighted::instruction::validate::build_validate_proof_ix(
        &payer.pubkey(),
        &params_account,
        &state_account,
    )?;

    // Prepare/ sign transaction.
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);

    // Push.
    banks_client.process_transaction(transaction).await?;

    Ok(())
}

#[tokio::test]
async fn test_validate_proof_invalid_message_hash() -> Result<()> {
    // Keys.
    let accounts_owner = Keypair::new();

    let params_account = Keypair::new().pubkey();
    let state_account = Keypair::new().pubkey();

    // Mock data prepare; params account.
    let (_, proof) = prepare_valid_proof();

    let invalid_message_hash: [u8; 32] = {
        let mut array = [0; 32];
        array.copy_from_slice(&hex::decode(
            "fb0609efd1dfeedfdcc8ba51520fae2d5176b7621d2560f071e801b0817e1537",
        )?);

        array
    };

    let params_account_b64 = general_purpose::STANDARD.encode(
        ValidateProofAccount {
            message_hash: invalid_message_hash,
            proof,
        }
        .pack(),
    );

    // Mock data prepare; program state.
    let state_account_b64 =
        general_purpose::STANDARD.encode(borsh::to_vec(&prepare_valid_state_account())?);

    // Env setup.
    let mut program_test: ProgramTest = program_test();

    // Add account with params; message_hash, proof.
    program_test.add_account_with_base64_data(
        params_account,
        999999,
        accounts_owner.pubkey(),
        &params_account_b64,
    );

    // Add account with state; message_hash, proof.
    program_test.add_account_with_base64_data(
        state_account,
        999999,
        accounts_owner.pubkey(),
        &state_account_b64,
    );

    // Kick.
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Prepare instruction; for tests only.
    let instruction = auth_weighted::instruction::validate::build_validate_proof_ix(
        &payer.pubkey(),
        &params_account,
        &state_account,
    )?;

    // Prepare/ sign transaction.
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(AuthWeightedError::MalformedSigners as u32)
        )
    );
    Ok(())
}
