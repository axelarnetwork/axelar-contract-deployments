use std::borrow::Cow;

use anyhow::{Ok, Result};
use ethers_core::abi::AbiEncode;
use gateway::events::GatewayEvent;
use interchain_token_service::events::InterchainTokenServiceEvent;
use interchain_token_service::{
    get_interchain_token_service_associated_token_account, MetadataVersion,
};
use interchain_token_transfer_gmp::{Bytes32, InterchainTransfer};
use solana_program::keccak::hash;
use solana_program::program_pack::Pack;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::sysvar::clock::Clock;
use solana_sdk::transaction::Transaction;
use spl_token::state::Account;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;
use token_manager::{get_token_flow_account, CalculatedEpoch, TokenManagerType};

use crate::program_test;

#[tokio::test]
async fn test_remote_interchain_transfer_mint_burn() -> Result<()> {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let token_id = [1u8; 32];
    let amount: u64 = 99;
    let destination_chain: Vec<u8> = b"ethereum".to_vec();
    let destination_address: Vec<u8> = vec![0, 1, 2, 3, 4, 5];
    let data: Vec<u8> = vec![0, 1, 2, 3];
    let symbol: Vec<u8> = vec![];

    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let delegate_authority = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_mint_preparations = 100;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
        .await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let mint_account_pda = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (owner_of_its_ata_for_user_tokens_pda, _) =
        get_interchain_token_service_associated_token_account(
            &interchain_token_service_root_pda,
            &delegate_authority.pubkey(),
            &mint_account_pda,
            &interchain_token_service::id(),
        )?;
    let its_ata_for_user_tokens_pda = spl_associated_token_account::get_associated_token_address(
        &owner_of_its_ata_for_user_tokens_pda,
        &mint_account_pda,
    );
    let groups = fixture
        .derive_token_manager_permission_groups(
            &Bytes32(token_id),
            &interchain_token_service_root_pda,
            &init_flow_limiter.pubkey(),
            &init_operator.pubkey(),
        )
        .await;

    fixture
        .setup_permission_group(&groups.flow_limiter_group)
        .await;
    fixture.setup_permission_group(&groups.operator_group).await;
    let token_manager_pda = fixture
        .setup_token_manager(
            token_manager::TokenManagerType::MintBurn,
            groups.clone(),
            500,
            gateway_root_pda,
            mint_account_pda,
            interchain_token_service_root_pda,
        )
        .await;

    // Fund as part of the preparations
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_mint_burn_instruction(
                amount_to_mint_preparations,
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_tokens_pda,
                &mint_account_pda,
                &delegate_authority.pubkey(),
                &gateway_root_pda,
                &gas_service_root_pda,
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_tokens_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount_to_mint_preparations,
        "incorrect amount after minting"
    );

    // Reference
    let payload = InterchainTransfer {
        token_id: Bytes32(token_id),
        source_address: fixture.payer.pubkey().to_bytes().to_vec(),
        destination_address: destination_address.clone(),
        amount: ethers_core::types::U256::from_little_endian(&amount.to_le_bytes()),
        data: data.clone(),
    }
    .encode();
    let payload_hash = hash(&payload).to_bytes();
    let data_hash = hash(&data).to_bytes();

    // Action
    let block_timestamp = fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .unix_timestamp;
    let token_manager_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_remote_interchain_transfer_mint_burn_instruction(
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_tokens_pda,
                &mint_account_pda,
                &delegate_authority.pubkey(),
                &gateway_root_pda,
                &gas_service_root_pda,
                &token_manager_pda,
                &token_manager_flow_pda,
                &groups.flow_limiter_group.group_pda,
                &groups.flow_limiter_group.group_pda_user,
                &groups.flow_limiter_group.group_pda_user_owner,
                &groups.operator_group.group_pda,
                &interchain_token_service_root_pda,
                token_id,
                destination_chain.clone(),
                destination_address.clone(),
                amount,
                data.clone(),
                MetadataVersion::ContractCall,
                symbol.clone(),
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &init_flow_limiter],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } = fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    // Assert
    assert!(
        result.is_ok(),
        "falied to process remote_interchain_transfer instruction"
    );

    let metadata = metadata
        .clone()
        .ok_or("transaction does not have a metadata")
        .unwrap();

    let interchain_token_service_event = metadata
        .log_messages
        .iter()
        .find_map(InterchainTokenServiceEvent::parse_log);

    assert_eq!(
        interchain_token_service_event,
        Some(InterchainTokenServiceEvent::InterchainTransfer {
            token_id,
            source_address: fixture.payer.pubkey().to_bytes().to_vec(),
            destination_chain: destination_chain.clone(),
            destination_address: destination_address.clone(),
            amount,
            hash: data_hash
        })
    );

    let gateway_event = metadata
        .log_messages
        .iter()
        .find_map(GatewayEvent::parse_log);

    assert_eq!(
        gateway_event,
        Some(GatewayEvent::CallContract(Cow::Owned(
            gateway::events::CallContract {
                sender: fixture.payer.pubkey(),
                destination_chain,
                destination_address,
                payload,
                payload_hash
            }
        )))
    );

    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_tokens_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount,
        amount_to_mint_preparations - amount,
        "incorrect amount after burning"
    );

    Ok(())
}

#[tokio::test]
async fn test_remote_interchain_transfer_lock_unlock() -> Result<()> {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let token_id = [1u8; 32];
    let amount: u64 = 100;
    let destination_chain: Vec<u8> = b"ethereum".to_vec();
    let destination_address: Vec<u8> = vec![0, 1, 2, 3, 4, 5];
    let data: Vec<u8> = vec![0, 1, 2, 3];
    let symbol: Vec<u8> = vec![];

    let _mint_authority = Keypair::new();
    let gateway_operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];

    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let destination = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&gateway_operators))
        .await;

    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let mint_account_pda = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (owner_of_its_ata_for_user_tokens_pda, _) =
        get_interchain_token_service_associated_token_account(
            &interchain_token_service_root_pda,
            &destination.pubkey(),
            &mint_account_pda,
            &interchain_token_service::id(),
        )?;
    let its_ata_for_user_tokens_pda = spl_associated_token_account::get_associated_token_address(
        &owner_of_its_ata_for_user_tokens_pda,
        &mint_account_pda,
    );
    let groups = fixture
        .derive_token_manager_permission_groups(
            &Bytes32(token_id),
            &interchain_token_service_root_pda,
            &init_flow_limiter.pubkey(),
            &init_operator.pubkey(),
        )
        .await;

    fixture
        .setup_permission_group(&groups.flow_limiter_group)
        .await;
    fixture.setup_permission_group(&groups.operator_group).await;
    let token_manager_pda = fixture
        .setup_token_manager(
            token_manager::TokenManagerType::LockUnlock,
            groups.clone(),
            500,
            gateway_root_pda,
            mint_account_pda,
            interchain_token_service_root_pda,
        )
        .await;

    // Fund as part of the preparations
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_mint_burn_instruction(
                amount,
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_tokens_pda,
                &mint_account_pda,
                &destination.pubkey(),
                &gateway_root_pda,
                &gas_service_root_pda,
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_tokens_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount,
        "incorrect amount after minting"
    );

    let (
        _token_manager_root_pda,
        token_manager_root_pda_account_data,
        _token_manager_handler_groups,
    ) = fixture
        .init_new_token_manager(
            interchain_token_service_root_pda,
            gas_service_root_pda,
            mint_account_pda,
            gateway_root_pda,
            TokenManagerType::LockUnlock,
            gateway_operators,
        )
        .await;

    // Reference
    let payload = InterchainTransfer {
        token_id: Bytes32(token_id),
        source_address: fixture.payer.pubkey().to_bytes().to_vec(),
        destination_address: destination_address.clone(),
        amount: ethers_core::types::U256::from_little_endian(&amount.to_le_bytes()),
        data: data.clone(),
    }
    .encode();
    let payload_hash = hash(&payload).to_bytes();
    let data_hash = hash(&data).to_bytes();

    // Action
    let block_timestamp = fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .unix_timestamp;
    let token_manager_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_remote_interchain_transfer_lock_unlock_instruction(
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &token_manager_root_pda_account_data.associated_token_account,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_tokens_pda,
                &mint_account_pda,
                &destination.pubkey(),
                &gateway_root_pda,
                &gas_service_root_pda,
                &token_manager_pda,
                &token_manager_flow_pda,
                &groups.flow_limiter_group.group_pda,
                &groups.flow_limiter_group.group_pda_user,
                &groups.flow_limiter_group.group_pda_user_owner,
                &groups.operator_group.group_pda,
                &interchain_token_service_root_pda,
                token_id,
                destination_chain.clone(),
                destination_address.clone(),
                amount,
                data.clone(),
                MetadataVersion::ContractCall,
                symbol.clone(),
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &init_flow_limiter],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } = fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    // Assert
    assert!(
        result.is_ok(),
        "falied to process remote_interchain_transfer instruction"
    );

    let metadata = metadata
        .clone()
        .ok_or("transaction does not have a metadata")
        .unwrap();

    let interchain_token_service_event = metadata
        .log_messages
        .iter()
        .find_map(InterchainTokenServiceEvent::parse_log);

    assert_eq!(
        interchain_token_service_event,
        Some(InterchainTokenServiceEvent::InterchainTransfer {
            token_id,
            source_address: fixture.payer.pubkey().to_bytes().to_vec(),
            destination_chain: destination_chain.clone(),
            destination_address: destination_address.clone(),
            amount,
            hash: data_hash
        })
    );

    let gateway_event = metadata
        .log_messages
        .iter()
        .find_map(GatewayEvent::parse_log);

    assert_eq!(
        gateway_event,
        Some(GatewayEvent::CallContract(Cow::Owned(
            gateway::events::CallContract {
                sender: fixture.payer.pubkey(),
                destination_chain,
                destination_address,
                payload,
                payload_hash
            }
        )))
    );

    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_tokens_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, 0,
        "ITS ATA still has tokens; should be zero"
    );

    let account = fixture
        .banks_client
        .get_account(token_manager_root_pda_account_data.associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount,
        "Token Manager ATA; incorrect balance"
    );

    Ok(())
}
