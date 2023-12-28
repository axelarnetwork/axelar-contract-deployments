mod common;

use std::collections::BTreeMap;

use ::base64::engine::general_purpose;
use anyhow::Result;
use auth_weighted::types::account::state::AuthWeightedStateAccount;
use auth_weighted::types::account::transfer_operatorship::TransferOperatorshipAccount;
use auth_weighted::types::address::Address;
use auth_weighted::types::u256::U256;
use base64::Engine;
use common::program_test;
use gateway::error::GatewayError;
use solana_program::instruction::InstructionError;
use solana_program::keccak;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};

#[tokio::test]
async fn test_transfer_operatorship_happy_scenario() -> Result<()> {
    let accounts_owner = gateway::id();
    let params_account = Keypair::new().pubkey();
    let (state_account, _bump) = Pubkey::find_program_address(&[&[]], &accounts_owner);

    let will_be_there = TransferOperatorshipAccount {
        operators: vec![
            Address::new(
                [
                    0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24,
                    0x2f, 0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98,
                    0x77, 0x29, 0x7d, 0x13, 0x16, 0xa0, 0x90,
                ]
                .into(),
            ),
            Address::new(
                [
                    0x03, 0xf5, 0x7d, 0x1a, 0x81, 0x3f, 0xeb, 0xac, 0xcb, 0xe6, 0x42, 0x96, 0x03,
                    0xf9, 0xec, 0x57, 0x96, 0x95, 0x11, 0xb7, 0x6c, 0xd6, 0x80, 0x45, 0x2d, 0xba,
                    0x91, 0xfa, 0x01, 0xf5, 0x4e, 0x75, 0x6d,
                ]
                .into(),
            ),
        ],
        weights: vec![U256::from(10), U256::from(91)],
        threshold: U256::from(100),
    };

    let is_already_there = TransferOperatorshipAccount {
        operators: vec![Address::new(
            [
                0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24, 0x2f,
                0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98, 0x77, 0x29,
                0x7d, 0x13, 0x16, 0xa0, 0x90,
            ]
            .into(),
        )],
        weights: vec![U256::from(100)],
        threshold: U256::from(10),
    };

    let params_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&will_be_there)?);

    // Prepare program state.
    let current_epoch = U256::from(1);

    let mut epoch_for_hash: BTreeMap<[u8; 32], U256> = BTreeMap::new();
    let mut hash_for_epoch: BTreeMap<U256, [u8; 32]> = BTreeMap::new();

    let operators_hash = keccak::hash(&borsh::to_vec(&is_already_there)?).to_bytes();

    epoch_for_hash.insert(operators_hash, current_epoch);
    hash_for_epoch.insert(current_epoch, operators_hash);

    let state_account_b64 =
        general_purpose::STANDARD.encode(borsh::to_vec(&AuthWeightedStateAccount {
            current_epoch,
            epoch_for_hash,
            hash_for_epoch,
        })?);

    let mut program_test: ProgramTest = program_test();

    program_test.add_account_with_base64_data(
        state_account,
        999999,
        accounts_owner,
        &state_account_b64,
    );

    program_test.add_account_with_base64_data(
        params_account,
        999999,
        accounts_owner,
        &params_account_b64,
    );

    //

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let state_data_after_before_mutation = banks_client
        .get_account(state_account)
        .await?
        .expect("there is an account");

    // Push.
    let instruction = gateway::instructions::transfer_operatorship(
        &payer.pubkey(),
        &params_account,
        &state_account,
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await?;

    // Checks.

    let state_data_after_mutation = banks_client
        .get_account(state_account)
        .await?
        .expect("there is an account");

    assert_ne!(
        state_data_after_mutation.data.len(),
        state_data_after_before_mutation.data.len()
    );

    let state_data_after_mutation_unpacked: AuthWeightedStateAccount =
        borsh::from_slice(&state_data_after_mutation.data)?;

    // Checks if current_epoch was mutated in the state.
    assert_eq!(
        state_data_after_mutation_unpacked.current_epoch,
        current_epoch + U256::from(1)
    );

    // TODO: check if epoch_for_hash is the valid one here.
    assert_eq!(
        state_data_after_mutation_unpacked
            .epoch_for_hash
            .get(&operators_hash),
        Some(U256::from(1)).as_ref(),
    );

    // TODO: check if hash_for_epoch is the valid one here.
    assert_eq!(
        state_data_after_mutation_unpacked
            .hash_for_epoch
            .get(&current_epoch.clone()),
        Some(operators_hash).as_ref(),
    );
    Ok(())
}

#[tokio::test]
async fn test_transfer_operatorship_duplicate_ops() -> Result<()> {
    let accounts_owner = gateway::id();
    let params_account = Keypair::new().pubkey();
    let (state_account, _bump) = Pubkey::find_program_address(&[&[]], &accounts_owner);

    let will_be_there = TransferOperatorshipAccount {
        operators: vec![
            Address::new(
                [
                    0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24,
                    0x2f, 0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98,
                    0x77, 0x29, 0x7d, 0x13, 0x16, 0xa0, 0x90,
                ]
                .into(),
            ),
            Address::new(
                [
                    0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24,
                    0x2f, 0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98,
                    0x77, 0x29, 0x7d, 0x13, 0x16, 0xa0, 0x90,
                ]
                .into(),
            ),
        ],
        weights: vec![U256::from(200)],
        threshold: U256::from(100),
    };

    let is_already_there = TransferOperatorshipAccount {
        operators: vec![Address::new(
            [
                0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24, 0x2f,
                0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98, 0x77, 0x29,
                0x7d, 0x13, 0x16, 0xa0, 0x90,
            ]
            .into(),
        )],
        weights: vec![U256::from(100)],
        threshold: U256::from(10),
    };

    let params_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&will_be_there)?);

    // Prepare program state.
    let current_epoch = U256::from(1);

    let mut epoch_for_hash: BTreeMap<[u8; 32], U256> = BTreeMap::new();
    let mut hash_for_epoch: BTreeMap<U256, [u8; 32]> = BTreeMap::new();

    let operators_hash = keccak::hash(&borsh::to_vec(&is_already_there)?).to_bytes();

    epoch_for_hash.insert(operators_hash, current_epoch);
    hash_for_epoch.insert(current_epoch, operators_hash);

    let state_account_b64 =
        general_purpose::STANDARD.encode(borsh::to_vec(&AuthWeightedStateAccount {
            current_epoch,
            epoch_for_hash,
            hash_for_epoch,
        })?);

    let mut program_test: ProgramTest = program_test();

    program_test.add_account_with_base64_data(
        state_account,
        999999,
        accounts_owner,
        &state_account_b64,
    );

    program_test.add_account_with_base64_data(
        params_account,
        999999,
        accounts_owner,
        &params_account_b64,
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let instruction = gateway::instructions::transfer_operatorship(
        &payer.pubkey(),
        &params_account,
        &state_account,
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(GatewayError::InvalidOperators as u32)
        )
    );
    Ok(())
}

#[tokio::test]
async fn test_transfer_operatorship_invald_weights() -> Result<()> {
    let accounts_owner = gateway::id();
    let params_account = Keypair::new().pubkey();
    let (state_account, _bump) = Pubkey::find_program_address(&[&[]], &accounts_owner);

    let will_be_there = TransferOperatorshipAccount {
        operators: vec![Address::new(
            [
                0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24, 0x2f,
                0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98, 0x77, 0x29,
                0x7d, 0x13, 0x16, 0xa0, 0x90,
            ]
            .into(),
        )],
        // TIP: There is more weights than operators.
        weights: vec![U256::from(10), U256::from(91)],
        threshold: U256::from(100),
    };

    let is_already_there = TransferOperatorshipAccount {
        operators: vec![Address::new(
            [
                0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24, 0x2f,
                0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98, 0x77, 0x29,
                0x7d, 0x13, 0x16, 0xa0, 0x90,
            ]
            .into(),
        )],
        weights: vec![U256::from(100)],
        threshold: U256::from(10),
    };

    let params_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&will_be_there)?);

    // Prepare program state.
    let current_epoch = U256::from(1);

    let mut epoch_for_hash: BTreeMap<[u8; 32], U256> = BTreeMap::new();
    let mut hash_for_epoch: BTreeMap<U256, [u8; 32]> = BTreeMap::new();

    let operators_hash = keccak::hash(&borsh::to_vec(&is_already_there)?).to_bytes();

    epoch_for_hash.insert(operators_hash, current_epoch);
    hash_for_epoch.insert(current_epoch, operators_hash);

    let state_account_b64 =
        general_purpose::STANDARD.encode(borsh::to_vec(&AuthWeightedStateAccount {
            current_epoch,
            epoch_for_hash,
            hash_for_epoch,
        })?);

    let mut program_test: ProgramTest = program_test();

    program_test.add_account_with_base64_data(
        state_account,
        999999,
        accounts_owner,
        &state_account_b64,
    );

    program_test.add_account_with_base64_data(
        params_account,
        999999,
        accounts_owner,
        &params_account_b64,
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let instruction = gateway::instructions::transfer_operatorship(
        &payer.pubkey(),
        &params_account,
        &state_account,
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(GatewayError::InvalidWeights as u32)
        )
    );
    Ok(())
}

#[tokio::test]
async fn test_transfer_operatorship_zero_weights() -> Result<()> {
    let accounts_owner = gateway::id();
    let params_account = Keypair::new().pubkey();
    let (state_account, _bump) = Pubkey::find_program_address(&[&[]], &accounts_owner);

    let will_be_there = TransferOperatorshipAccount {
        operators: vec![
            Address::new(
                [
                    0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24,
                    0x2f, 0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98,
                    0x77, 0x29, 0x7d, 0x13, 0x16, 0xa0, 0x90,
                ]
                .into(),
            ),
            Address::new(
                [
                    0x03, 0xf5, 0x7d, 0x1a, 0x81, 0x3f, 0xeb, 0xac, 0xcb, 0xe6, 0x42, 0x96, 0x03,
                    0xf9, 0xec, 0x57, 0x96, 0x95, 0x11, 0xb7, 0x6c, 0xd6, 0x80, 0x45, 0x2d, 0xba,
                    0x91, 0xfa, 0x01, 0xf5, 0x4e, 0x75, 0x6d,
                ]
                .into(),
            ),
        ],
        // TIP: There is NO weights.
        weights: vec![],
        threshold: U256::from(100),
    };

    let is_already_there = TransferOperatorshipAccount {
        operators: vec![Address::new(
            [
                0x02, 0xd1, 0xe0, 0xcf, 0xf6, 0x3a, 0xa3, 0xe7, 0x98, 0x8e, 0x40, 0x70, 0x24, 0x2f,
                0xa3, 0x78, 0x71, 0xa9, 0xab, 0xc7, 0x9e, 0xcf, 0x85, 0x1c, 0xce, 0x98, 0x77, 0x29,
                0x7d, 0x13, 0x16, 0xa0, 0x90,
            ]
            .into(),
        )],
        // TIP: There is NO weights.
        weights: vec![],
        threshold: U256::from(10),
    };

    let params_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&will_be_there)?);

    // Prepare program state.
    let current_epoch = U256::from(1);

    let mut epoch_for_hash: BTreeMap<[u8; 32], U256> = BTreeMap::new();
    let mut hash_for_epoch: BTreeMap<U256, [u8; 32]> = BTreeMap::new();

    let operators_hash = keccak::hash(&borsh::to_vec(&is_already_there)?).to_bytes();

    epoch_for_hash.insert(operators_hash, current_epoch);
    hash_for_epoch.insert(current_epoch, operators_hash);

    let state_account_b64 =
        general_purpose::STANDARD.encode(borsh::to_vec(&AuthWeightedStateAccount {
            current_epoch,
            epoch_for_hash,
            hash_for_epoch,
        })?);

    let mut program_test: ProgramTest = program_test();

    program_test.add_account_with_base64_data(
        state_account,
        999999,
        accounts_owner,
        &state_account_b64,
    );

    program_test.add_account_with_base64_data(
        params_account,
        999999,
        accounts_owner,
        &params_account_b64,
    );

    //

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    // Push.
    let instruction = gateway::instructions::transfer_operatorship(
        &payer.pubkey(),
        &params_account,
        &state_account,
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    assert_eq!(
        banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(GatewayError::InvalidWeights as u32)
        )
    );
    Ok(())
}
