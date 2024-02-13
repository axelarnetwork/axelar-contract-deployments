#![cfg(test)]

mod common;

use ::base64::engine::general_purpose;
use anyhow::Result;
use base64::Engine;
use common::program_test;
use gmp_gateway::accounts::transfer_operatorship::TransferOperatorshipAccount;
use gmp_gateway::accounts::GatewayConfig;
use gmp_gateway::error::GatewayError;
use gmp_gateway::types::address::Address;
use gmp_gateway::types::u256::U256;
use solana_program::instruction::InstructionError;
use solana_program::keccak;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::signature::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};
use test_fixtures::primitives::bytes;

#[tokio::test]
async fn test_transfer_operatorship_happy_scenario() -> Result<()> {
    let accounts_owner = gmp_gateway::id();
    let (state_account_address, _bump) = Pubkey::find_program_address(&[&[]], &accounts_owner);

    // Existing worker set
    let mut addresses = [bytes(33), bytes(33)];
    addresses.sort();
    let existing_operators_and_weights: Vec<(Address, U256)> = vec![
        (Address::try_from(&*addresses[0])?, 10u8.into()),
        (Address::try_from(&*addresses[1])?, 91u8.into()),
    ];

    let will_be_there =
        TransferOperatorshipAccount::new(existing_operators_and_weights, 100u8.into());
    let params_account = will_be_there.pda().0;

    // Proposed worker set
    let proposed_operators_and_weights: Vec<(Address, U256)> = vec![(
        "02d1e0cff63aa3e7988e4070242fa37871a9abc79ecf851cce9877297d1316a090".try_into()?,
        100u8.into(),
    )];
    let is_already_there =
        TransferOperatorshipAccount::new(proposed_operators_and_weights, 10u8.into());

    let params_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&will_be_there)?);

    // Prepare operator state.
    let current_epoch = U256::ONE;
    let operators_hash = keccak::hash(&borsh::to_vec(&is_already_there)?).to_bytes();
    let mut gateway_config = GatewayConfig::default();
    gateway_config.operators_and_epochs.update(operators_hash)?;
    let state_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&gateway_config)?);

    let mut program_test: ProgramTest = program_test();

    program_test.add_account_with_base64_data(
        state_account_address,
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
        .get_account(state_account_address)
        .await?
        .expect("there is an account");

    // Push.
    let instruction = gmp_gateway::instructions::transfer_operatorship(
        &payer.pubkey(),
        &params_account,
        &state_account_address,
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
        .get_account(state_account_address)
        .await?
        .expect("there is an account");

    assert_ne!(
        state_data_after_mutation.data.len(),
        state_data_after_before_mutation.data.len()
    );

    let state_data_after_mutation_unpacked: GatewayConfig =
        borsh::from_slice(&state_data_after_mutation.data)?;

    // Checks if current_epoch was mutated in the state.
    assert_eq!(
        state_data_after_mutation_unpacked
            .operators_and_epochs
            .current_epoch(),
        current_epoch
            .checked_add(U256::ONE)
            .expect("arithmetic overflow")
    );

    // TODO: check if epoch_for_hash is the valid one here.
    assert_eq!(
        state_data_after_mutation_unpacked
            .operators_and_epochs
            .epoch_for_operator_hash(&operators_hash),
        Some(U256::ONE).as_ref(),
    );

    // TODO: check if hash_for_epoch is the valid one here.
    assert_eq!(
        state_data_after_mutation_unpacked
            .operators_and_epochs
            .operator_hash_for_epoch(&current_epoch),
        Some(operators_hash).as_ref(),
    );
    Ok(())
}

#[tokio::test]
async fn test_transfer_operatorship_duplicate_ops() -> Result<()> {
    let accounts_owner = gmp_gateway::id();
    let (state_account_address, _bump) = Pubkey::find_program_address(&[&[]], &accounts_owner);

    let duplicated_operator: Address = bytes(33).as_slice().try_into()?;
    let proposed_operator_and_weights = vec![
        (duplicated_operator, 200u8.into()),
        (duplicated_operator, 15u8.into()),
    ];
    let will_be_there =
        TransferOperatorshipAccount::new(proposed_operator_and_weights, 100u8.into());
    let (params_account, _) = will_be_there.pda();

    let is_already_there = TransferOperatorshipAccount::new(
        vec![(bytes(33).as_slice().try_into()?, 100u8.into())],
        10u8.into(),
    );

    let params_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&will_be_there)?);

    // Prepare operator state.
    let operators_hash = keccak::hash(&borsh::to_vec(&is_already_there)?).to_bytes();
    let mut gateway_config = GatewayConfig::default();
    gateway_config.operators_and_epochs.update(operators_hash)?;
    let state_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&gateway_config)?);

    let mut program_test: ProgramTest = program_test();

    program_test.add_account_with_base64_data(
        state_account_address,
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

    let instruction = gmp_gateway::instructions::transfer_operatorship(
        &payer.pubkey(),
        &params_account,
        &state_account_address,
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
            InstructionError::Custom(dbg!(GatewayError::UnorderedOrDuplicateOperators as u32))
        )
    );
    Ok(())
}

#[tokio::test]
async fn test_transfer_operatorship_zero_threshold() -> Result<()> {
    let accounts_owner = gmp_gateway::id();
    let (state_account_address, _bump) = Pubkey::find_program_address(&[&[]], &accounts_owner);

    let operator_with_invalid_weight = vec![(bytes(33).as_slice().try_into()?, 150u8.into())];
    let will_be_there = TransferOperatorshipAccount::new(operator_with_invalid_weight, 0u8.into());
    let (params_account, _) = will_be_there.pda();

    let is_already_there = TransferOperatorshipAccount::new(
        vec![(bytes(33).as_slice().try_into()?, 100u8.into())],
        10u8.into(),
    );

    let params_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&will_be_there)?);

    // Prepare operator state.
    let operators_hash = keccak::hash(&borsh::to_vec(&is_already_there)?).to_bytes();
    let mut gateway_config = GatewayConfig::default();
    gateway_config.operators_and_epochs.update(operators_hash)?;
    let state_account_b64 = general_purpose::STANDARD.encode(borsh::to_vec(&gateway_config)?);

    let mut program_test: ProgramTest = program_test();

    program_test.add_account_with_base64_data(
        state_account_address,
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

    let instruction = gmp_gateway::instructions::transfer_operatorship(
        &payer.pubkey(),
        &params_account,
        &state_account_address,
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
            InstructionError::Custom(GatewayError::ZeroThreshold as u32)
        )
    );
    Ok(())
}
