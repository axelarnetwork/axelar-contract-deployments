mod common;

use anyhow::{Ok, Result};
use auth_weighted::types::u256::U256;
use base64::engine::general_purpose;
use base64::Engine;
use borsh::BorshDeserialize;
use common::program_test;
use gas_service::accounts::GasServiceRootPDA;
use gas_service::error::GasServiceError;
use gas_service::events::GasServiceEvent;
use gateway::types::PubkeyWrapper;
use solana_program::instruction::InstructionError;
use solana_program::keccak::hash;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata, ProgramTest};
use solana_sdk::account::Account;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};

#[tokio::test]
async fn init_root_pda() -> Result<()> {
    let (mut banks_client, initializer_account, recent_blockhash) = program_test().start().await;

    let (root_pda_address, _) = gas_service::get_gas_service_root_pda();

    let ix = gas_service::instruction::create_initialize_root_pda_ix(initializer_account.pubkey())?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    let root_pda_data = banks_client.get_account(root_pda_address).await?.unwrap();
    let root_pda_data =
        gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())?;

    assert!(root_pda_data.check_authority(initializer_account.pubkey()));

    Ok(())
}

#[tokio::test]
async fn pay_native_gas_for_contract_call_happy_scenario() -> Result<()> {
    let refund_address = Keypair::new();
    let initializer_address = Keypair::new();
    let (root_pda_address, _bump) = gas_service::get_gas_service_root_pda();

    let mut program_test: ProgramTest = program_test();

    let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
        initializer_address.pubkey(),
    )))?;

    let serialized_data = general_purpose::STANDARD.encode(serialized_data);

    let initial_saldo = 999999;
    program_test.add_account_with_base64_data(
        root_pda_address,
        initial_saldo,
        gas_service::id(),
        &serialized_data,
    );

    let (mut banks_client, sender, recent_blockhash) = program_test.start().await;

    // Check: pre-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        initial_saldo
    );

    let destination_chain = "ethereum".to_string().into_bytes();
    let destination_address = b"0xb794f5ea0ba39494ce839613fffba74279579268".to_vec();
    let payload = b"some payload".to_vec();
    let fees = 100000000; // 1 sol in lamports;

    let ix = gas_service::instruction::create_pay_native_gas_for_contract_call_ix(
        sender.pubkey(),
        refund_address.pubkey(),
        destination_chain.clone(),
        destination_address.clone(),
        payload.clone(),
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&sender.pubkey()),
        &[&sender],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } =
        banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let from_meta = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GasServiceEvent::parse_log)
        .next();

    assert_eq!(
        from_meta,
        Some(GasServiceEvent::NativeGasPaidForContractCall {
            sender: PubkeyWrapper::from(sender.pubkey()),
            destination_chain,
            destination_address,
            payload_hash: hash(&payload).to_bytes(),
            fees,
            refund_address: PubkeyWrapper::from(refund_address.pubkey()),
        })
    );

    // Check: after-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        fees + initial_saldo
    );

    Ok(())
}

#[tokio::test]
async fn pay_native_gas_for_contract_call_with_token_happy_scenario() -> Result<()> {
    let refund_address = Keypair::new();
    let initializer_address = Keypair::new();
    let (root_pda_address, _bump) = gas_service::get_gas_service_root_pda();

    let mut program_test: ProgramTest = program_test();

    let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
        initializer_address.pubkey(),
    )))?;

    let serialized_data = general_purpose::STANDARD.encode(serialized_data);

    let initial_saldo = 999999;
    program_test.add_account_with_base64_data(
        root_pda_address,
        initial_saldo,
        gas_service::id(),
        &serialized_data,
    );

    let (mut banks_client, sender, recent_blockhash) = program_test.start().await;

    // Check: pre-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        initial_saldo
    );

    let destination_chain = "ethereum".to_string().into_bytes();
    let destination_address = b"0xb794f5ea0ba39494ce839613fffba74279579268".to_vec();
    let token_symbol = b"ETH".to_vec();
    let token_amount = U256::ONE;
    let payload = b"some payload".to_vec();
    let fees = 100000000; // 1 sol in lamports;

    let ix = gas_service::instruction::create_pay_native_gas_for_contract_call_with_token_ix(
        sender.pubkey(),
        refund_address.pubkey(),
        destination_chain.clone(),
        destination_address.clone(),
        payload.clone(),
        token_symbol.clone(),
        token_amount,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&sender.pubkey()),
        &[&sender],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } =
        banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let from_meta = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GasServiceEvent::parse_log)
        .next();

    assert_eq!(
        from_meta,
        Some(GasServiceEvent::NativeGasPaidForContractCallWithToken {
            sender: PubkeyWrapper::from(sender.pubkey()),
            destination_chain,
            destination_address,
            payload_hash: hash(&payload).to_bytes(),
            symbol: token_symbol.clone(),
            amount: token_amount,
            fees,
            refund_address: PubkeyWrapper::from(refund_address.pubkey()),
        })
    );

    // Check: after-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        fees + initial_saldo
    );

    Ok(())
}

#[tokio::test]
async fn pay_native_gas_for_express_call_happy_scenario() -> Result<()> {
    let refund_address = Keypair::new();
    let initializer_address = Keypair::new();
    let (root_pda_address, _bump) = gas_service::get_gas_service_root_pda();

    let mut program_test: ProgramTest = program_test();

    let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
        initializer_address.pubkey(),
    )))?;

    let serialized_data = general_purpose::STANDARD.encode(serialized_data);

    let initial_saldo = 999999;
    program_test.add_account_with_base64_data(
        root_pda_address,
        initial_saldo,
        gas_service::id(),
        &serialized_data,
    );

    let (mut banks_client, sender, recent_blockhash) = program_test.start().await;

    // Check: pre-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        initial_saldo
    );

    let destination_chain = "ethereum".to_string().into_bytes();
    let destination_address = b"0xb794f5ea0ba39494ce839613fffba74279579268".to_vec();
    let payload = b"some payload".to_vec();
    let fees = 100000000; // 1 sol in lamports;

    let ix = gas_service::instruction::create_pay_native_gas_for_express_call_ix(
        sender.pubkey(),
        refund_address.pubkey(),
        destination_chain.clone(),
        destination_address.clone(),
        payload.clone(),
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&sender.pubkey()),
        &[&sender],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } =
        banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let from_meta = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GasServiceEvent::parse_log)
        .next();

    assert_eq!(
        from_meta,
        Some(GasServiceEvent::NativeGasPaidForExpressCall {
            sender: PubkeyWrapper::from(sender.pubkey()),
            destination_chain,
            destination_address,
            payload_hash: hash(&payload).to_bytes(),
            fees,
            refund_address: PubkeyWrapper::from(refund_address.pubkey()),
        })
    );

    // Check: after-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        fees + initial_saldo
    );

    Ok(())
}

#[tokio::test]
async fn pay_native_gas_for_express_call_with_token_happy_scenario() -> Result<()> {
    let refund_address = Keypair::new();
    let initializer_address = Keypair::new();
    let (root_pda_address, _bump) = gas_service::get_gas_service_root_pda();

    let mut program_test: ProgramTest = program_test();

    let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
        initializer_address.pubkey(),
    )))?;

    let serialized_data = general_purpose::STANDARD.encode(serialized_data);

    let initial_saldo = 999999;
    program_test.add_account_with_base64_data(
        root_pda_address,
        initial_saldo,
        gas_service::id(),
        &serialized_data,
    );

    let (mut banks_client, sender, recent_blockhash) = program_test.start().await;

    // Check: pre-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        initial_saldo
    );

    let destination_chain = "ethereum".to_string().into_bytes();
    let destination_address = b"0xb794f5ea0ba39494ce839613fffba74279579268".to_vec();
    let token_symbol = b"ETH".to_vec();
    let token_amount = U256::ONE;
    let payload = b"some payload".to_vec();
    let fees = 100000000; // 1 sol in lamports;

    let ix = gas_service::instruction::create_pay_native_gas_for_express_call_with_token_ix(
        sender.pubkey(),
        refund_address.pubkey(),
        destination_chain.clone(),
        destination_address.clone(),
        payload.clone(),
        token_symbol.clone(),
        token_amount,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&sender.pubkey()),
        &[&sender],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } =
        banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let from_meta = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GasServiceEvent::parse_log)
        .next();

    assert_eq!(
        from_meta,
        Some(GasServiceEvent::NativeGasPaidForExpressCallWithToken {
            sender: PubkeyWrapper::from(sender.pubkey()),
            destination_chain,
            destination_address,
            payload_hash: hash(&payload).to_bytes(),
            symbol: token_symbol.clone(),
            amount: token_amount,
            fees,
            refund_address: PubkeyWrapper::from(refund_address.pubkey()),
        })
    );

    // Check: after-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        fees + initial_saldo
    );

    Ok(())
}

#[tokio::test]
async fn add_native_gas_happy_scenario() -> Result<()> {
    let refund_address = Keypair::new();
    let initializer_address = Keypair::new();
    let (root_pda_address, _bump) = gas_service::get_gas_service_root_pda();

    let mut program_test: ProgramTest = program_test();

    let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
        initializer_address.pubkey(),
    )))?;

    let serialized_data = general_purpose::STANDARD.encode(serialized_data);

    let initial_saldo = 999999;
    program_test.add_account_with_base64_data(
        root_pda_address,
        initial_saldo,
        gas_service::id(),
        &serialized_data,
    );

    let (mut banks_client, sender, recent_blockhash) = program_test.start().await;

    // Check: pre-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        initial_saldo
    );

    let tx_hash = [1u8; 64];
    let log_index = 0;
    let fees = 100000000; // 1 sol in lamports;

    let ix = gas_service::instruction::create_add_native_gas_ix(
        sender.pubkey(),
        refund_address.pubkey(),
        tx_hash,
        log_index,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&sender.pubkey()),
        &[&sender],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } =
        banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let from_meta = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GasServiceEvent::parse_log)
        .next();

    assert_eq!(
        from_meta,
        Some(GasServiceEvent::NativeGasAdded {
            tx_hash,
            log_index,
            fees,
            refund_address: PubkeyWrapper::from(refund_address.pubkey()),
        })
    );

    // Check: after-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        fees + initial_saldo
    );

    Ok(())
}

#[tokio::test]
async fn add_native_express_gas_happy_scenario() -> Result<()> {
    let refund_address = Keypair::new();
    let initializer_address = Keypair::new();
    let (root_pda_address, _bump) = gas_service::get_gas_service_root_pda();

    let mut program_test: ProgramTest = program_test();

    let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
        initializer_address.pubkey(),
    )))?;

    let serialized_data = general_purpose::STANDARD.encode(serialized_data);

    let initial_saldo = 999999;
    program_test.add_account_with_base64_data(
        root_pda_address,
        initial_saldo,
        gas_service::id(),
        &serialized_data,
    );

    let (mut banks_client, sender, recent_blockhash) = program_test.start().await;

    // Check: pre-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        initial_saldo
    );

    let tx_hash = [1u8; 64];
    let log_index = 0;
    let fees = 100000000; // 1 sol in lamports;

    let ix = gas_service::instruction::create_add_native_express_gas_ix(
        sender.pubkey(),
        refund_address.pubkey(),
        tx_hash,
        log_index,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&sender.pubkey()),
        &[&sender],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } =
        banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let from_meta = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GasServiceEvent::parse_log)
        .next();

    assert_eq!(
        from_meta,
        Some(GasServiceEvent::NativeExpressGasAdded {
            tx_hash,
            log_index,
            fees,
            refund_address: PubkeyWrapper::from(refund_address.pubkey()),
        })
    );

    // Check: after-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        fees + initial_saldo
    );

    Ok(())
}

#[tokio::test]
async fn collect_fees_unauthorized() -> Result<()> {
    let receiver_address = Keypair::new();
    let initializer_address = Keypair::new();
    let (root_pda_address, _bump) = gas_service::get_gas_service_root_pda();

    let mut program_test: ProgramTest = program_test();

    let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
        initializer_address.pubkey(),
    )))?;

    let serialized_data = general_purpose::STANDARD.encode(serialized_data);

    let initial_saldo = 999999;
    program_test.add_account_with_base64_data(
        root_pda_address,
        initial_saldo,
        gas_service::id(),
        &serialized_data,
    );

    let (mut banks_client, _banks_signer, recent_blockhash) = program_test.start().await;

    // Check: pre-payment saldo.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        initial_saldo
    );

    let amount_to_collect = 100000000; // 1 sol in lamports;

    let ix = gas_service::instruction::create_collect_fees_ix(
        _banks_signer.pubkey(),
        receiver_address.pubkey(),
        amount_to_collect,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&_banks_signer.pubkey()),
        &[&_banks_signer],
        recent_blockhash,
    );

    assert_eq!(
        banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(GasServiceError::SenderAccountIsNotExpectedAuthority as u32)
        )
    );

    Ok(())
}

#[tokio::test]
async fn collect_fees_happy_scenario() -> Result<()> {
    let receiver_address = Keypair::new();
    let (root_pda_address, _) = gas_service::get_gas_service_root_pda();
    let mut program_test: ProgramTest = program_test();

    let receiver_saldo = 1000000;
    program_test.add_account(
        receiver_address.pubkey(),
        Account::new(receiver_saldo, 1, &receiver_address.pubkey()),
    );

    let (mut banks_client, initializer_account, recent_blockhash) = program_test.start().await;

    // Initialize.
    let ix = gas_service::instruction::create_initialize_root_pda_ix(initializer_account.pubkey())?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    let rent_exempt = 1113600;

    let root_pda_data = banks_client.get_account(root_pda_address).await?.unwrap();
    let root_pda_data =
        gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())?;

    // Check: For initializer as authority.
    assert!(root_pda_data.check_authority(initializer_account.pubkey()));

    // Fund the root PDA account.
    let tx_hash = [1u8; 64];
    let log_index = 0;
    let fees = 999999999999;

    let ix = gas_service::instruction::create_add_native_express_gas_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        tx_hash,
        log_index,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    // Check: account was paid.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        rent_exempt + fees
    );

    // Collect Fees.
    let amount_to_collect = 100;

    let actual_ix = gas_service::instruction::create_collect_fees_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        amount_to_collect,
    )?;

    let actual_tx = Transaction::new_signed_with_payer(
        &[actual_ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(actual_tx).await?;

    // Check: Amount was deducted.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        (rent_exempt + fees) - amount_to_collect
    );

    // Check: Reciever got funds.
    assert_eq!(
        banks_client
            .get_account(receiver_address.pubkey())
            .await?
            .unwrap()
            .lamports,
        amount_to_collect + receiver_saldo
    );

    Ok(())
}

#[tokio::test]
async fn collect_fees_collect_more_than_could_be() -> Result<()> {
    let receiver_address = Keypair::new();
    let (root_pda_address, _) = gas_service::get_gas_service_root_pda();
    let mut program_test: ProgramTest = program_test();

    let receiver_saldo = 1000000;
    program_test.add_account(
        receiver_address.pubkey(),
        Account::new(receiver_saldo, 1, &receiver_address.pubkey()),
    );

    let (mut banks_client, initializer_account, recent_blockhash) = program_test.start().await;

    // Initialize.
    let ix = gas_service::instruction::create_initialize_root_pda_ix(initializer_account.pubkey())?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    let rent_exempt = 1113600;

    let root_pda_data = banks_client.get_account(root_pda_address).await?.unwrap();
    let root_pda_data =
        gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())?;

    // Check: For initializer as authority.
    assert!(root_pda_data.check_authority(initializer_account.pubkey()));

    // Fund the root PDA account.
    let tx_hash = [1u8; 64];
    let log_index = 0;
    let fees = 999999999999;

    let ix = gas_service::instruction::create_add_native_express_gas_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        tx_hash,
        log_index,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    // Check: account was paid.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        rent_exempt + fees
    );

    // Collect Fees.
    let amount_to_collect = 9999999999999999999;

    let actual_ix = gas_service::instruction::create_collect_fees_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        amount_to_collect,
    )?;

    let actual_tx = Transaction::new_signed_with_payer(
        &[actual_ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    assert_eq!(
        banks_client
            .process_transaction(actual_tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(GasServiceError::InsufficientFundsForTransaction as u32)
        )
    );

    Ok(())
}

#[tokio::test]
async fn refund_happy_scenario() -> Result<()> {
    let receiver_address = Keypair::new();
    let (root_pda_address, _) = gas_service::get_gas_service_root_pda();
    let mut program_test: ProgramTest = program_test();

    let receiver_saldo = 1000000;
    program_test.add_account(
        receiver_address.pubkey(),
        Account::new(receiver_saldo, 1, &receiver_address.pubkey()),
    );

    let (mut banks_client, initializer_account, recent_blockhash) = program_test.start().await;

    // Initialize.
    let ix = gas_service::instruction::create_initialize_root_pda_ix(initializer_account.pubkey())?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    let rent_exempt = 1113600;

    let root_pda_data = banks_client.get_account(root_pda_address).await?.unwrap();
    let root_pda_data =
        gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())?;

    // Check: For initializer as authority.
    assert!(root_pda_data.check_authority(initializer_account.pubkey()));

    // Fund the root PDA account.
    let tx_hash = [1u8; 64];
    let log_index = 0;
    let fees = 999999999999;

    let ix = gas_service::instruction::create_add_native_express_gas_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        tx_hash,
        log_index,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    // Check: account was paid.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        rent_exempt + fees
    );

    // Collect Fees.
    let amount_to_collect = 100;

    let actual_ix = gas_service::instruction::create_refund_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        amount_to_collect,
    )?;

    let actual_tx = Transaction::new_signed_with_payer(
        &[actual_ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(actual_tx).await?;

    // Check: Amount was deducted.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        (rent_exempt + fees) - amount_to_collect
    );

    // Check: Reciever got funds.
    assert_eq!(
        banks_client
            .get_account(receiver_address.pubkey())
            .await?
            .unwrap()
            .lamports,
        amount_to_collect + receiver_saldo
    );

    Ok(())
}

#[tokio::test]
async fn refund_more_than_could_be() -> Result<()> {
    let receiver_address = Keypair::new();
    let (root_pda_address, _) = gas_service::get_gas_service_root_pda();
    let mut program_test: ProgramTest = program_test();

    let receiver_saldo = 1000000;
    program_test.add_account(
        receiver_address.pubkey(),
        Account::new(receiver_saldo, 1, &receiver_address.pubkey()),
    );

    let (mut banks_client, initializer_account, recent_blockhash) = program_test.start().await;

    // Initialize.
    let ix = gas_service::instruction::create_initialize_root_pda_ix(initializer_account.pubkey())?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    let rent_exempt = 1113600;

    let root_pda_data = banks_client.get_account(root_pda_address).await?.unwrap();
    let root_pda_data =
        gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())?;

    // Check: For initializer as authority.
    assert!(root_pda_data.check_authority(initializer_account.pubkey()));

    // Fund the root PDA account.
    let tx_hash = [1u8; 64];
    let log_index = 0;
    let fees = 999999999999;

    let ix = gas_service::instruction::create_add_native_express_gas_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        tx_hash,
        log_index,
        fees,
    )?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    banks_client.process_transaction(tx).await?;

    // Check: account was paid.
    assert_eq!(
        banks_client
            .get_account(root_pda_address)
            .await?
            .unwrap()
            .lamports,
        rent_exempt + fees
    );

    // Collect Fees.
    let amount_to_collect = 9999999999999999999;

    let actual_ix = gas_service::instruction::create_refund_ix(
        initializer_account.pubkey(),
        receiver_address.pubkey(),
        amount_to_collect,
    )?;

    let actual_tx = Transaction::new_signed_with_payer(
        &[actual_ix],
        Some(&initializer_account.pubkey()),
        &[&initializer_account],
        recent_blockhash,
    );

    assert_eq!(
        banks_client
            .process_transaction(actual_tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(GasServiceError::InsufficientFundsForTransaction as u32)
        )
    );

    Ok(())
}
