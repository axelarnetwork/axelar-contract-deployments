use anyhow::anyhow;
use axelar_solana_gateway_test_fixtures::assert_msg_present_in_logs;
use axelar_solana_its::state::token_manager::TokenManager;
use borsh::BorshDeserialize;
use event_utils::Event;
use evm_contracts_test_suite::ethers::signers::Signer;
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::program_pack::Pack;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token_2022::extension::transfer_fee::TransferFeeConfig;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::{Account, Mint};
use test_context::test_context;

use crate::{fetch_first_call_contract_event_from_tx, ItsTestContext};

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_canonical_token_with_fee_lock_unlock(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    // Create a canonical token with TransferFeeConfig extension (10% fee)
    let fee_basis_points = 1000_u16; // 10%
    let maximum_fee = u64::MAX;
    let canonical_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint_with_fee(
            &ctx.solana_wallet,
            &spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            9,
            None,
            None,
        )
        .await;

    // Create Metaplex metadata for the canonical token
    let (canonical_metadata_pda, _) = Metadata::find_pda(&canonical_mint);
    let create_metadata_ix = CreateV1Builder::new()
        .metadata(canonical_metadata_pda)
        .mint(canonical_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Canonical Fee Token".to_string())
        .symbol("CFT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_metadata_ix]).await.unwrap();

    // Register the canonical token metadata
    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    // Deploy remote canonical token (this creates the token manager)
    let deploy_remote_canonical_ix =
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            ctx.evm_chain_name.clone(),
            0,
        )?;

    let tx = ctx
        .send_solana_tx(&[deploy_remote_canonical_ix])
        .await
        .unwrap();

    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);

    // Relay to EVM to establish the token
    ctx.relay_to_evm(&call_contract_event.payload).await;

    // Get the canonical token ID
    let canonical_token_id = axelar_solana_its::canonical_interchain_token_id(&canonical_mint);

    // Create user account and give them tokens for test transfer
    let user_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &canonical_mint,
        &spl_token_2022::id(),
    );
    let create_user_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &canonical_mint,
        &spl_token_2022::id(),
    );
    let user_balance = 10000_u64;
    let mint_to_user_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &canonical_mint,
        &user_ata,
        &ctx.solana_wallet,
        &[],
        user_balance,
    )?;

    ctx.send_solana_tx(&[create_user_ata_ix, mint_to_user_ix])
        .await
        .unwrap();

    // Test transfer
    let transfer_amount = 1000_u64;
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        canonical_token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        canonical_mint,
        spl_token_2022::id(),
        0,
    )?;

    let transfer_tx = ctx.send_solana_tx(&[transfer_ix]).await.unwrap();

    // Verify fee calculation
    let mint_data = ctx
        .solana_chain
        .try_get_account_no_checks(&canonical_mint)
        .await
        .unwrap()
        .unwrap();
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data.data).unwrap();
    let fee_config = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    let clock_sysvar = ctx.solana_chain.get_sysvar::<Clock>().await;
    let epoch = clock_sysvar.epoch;
    let fee = fee_config
        .calculate_epoch_fee(epoch, transfer_amount)
        .unwrap();

    let transfer_logs = transfer_tx.metadata.unwrap().log_messages;
    let transfer_event = transfer_logs
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTransfer::try_from_log(log).ok())
        .unwrap();

    let amount_after_fee = transfer_amount.checked_sub(fee).unwrap();
    assert_eq!(transfer_event.amount, amount_after_fee);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_canonical_token_various_fee_configs(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    // Test with smaller fee (0.5%)
    let fee_basis_points = 50_u16;
    let maximum_fee = 1000_u64;
    let canonical_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint_with_fee(
            &ctx.solana_wallet,
            &spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            6,
            None,
            None,
        )
        .await;

    // Create Metaplex metadata for the canonical token
    let (canonical_metadata_pda, _) = Metadata::find_pda(&canonical_mint);
    let create_metadata_ix = CreateV1Builder::new()
        .metadata(canonical_metadata_pda)
        .mint(canonical_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Low Fee Canonical Token".to_string())
        .symbol("LFCT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(6)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_metadata_ix]).await.unwrap();

    // Register and deploy the canonical token
    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    let deploy_remote_canonical_ix =
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            ctx.evm_chain_name.clone(),
            0,
        )?;

    let tx = ctx
        .send_solana_tx(&[deploy_remote_canonical_ix])
        .await
        .unwrap();

    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);
    ctx.relay_to_evm(&call_contract_event.payload).await;

    // Test transfer with this fee configuration
    let canonical_token_id = axelar_solana_its::canonical_interchain_token_id(&canonical_mint);

    // Set up user account
    let user_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &canonical_mint,
        &spl_token_2022::id(),
    );
    let create_user_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &canonical_mint,
        &spl_token_2022::id(),
    );

    let initial_balance = 100_000_u64;
    let mint_to_user_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &canonical_mint,
        &user_ata,
        &ctx.solana_wallet,
        &[],
        initial_balance,
    )?;

    ctx.send_solana_tx(&[create_user_ata_ix, mint_to_user_ix])
        .await
        .unwrap();

    // Test transfer
    let transfer_amount = 10_000_u64;
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        canonical_token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        canonical_mint,
        spl_token_2022::id(),
        0,
    )?;

    let transfer_tx = ctx.send_solana_tx(&[transfer_ix]).await.unwrap();

    // Verify fee calculation with lower fee rate
    let mint_data = ctx
        .solana_chain
        .try_get_account_no_checks(&canonical_mint)
        .await
        .unwrap()
        .unwrap();
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data.data).unwrap();
    let fee_config = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    let clock_sysvar = ctx.solana_chain.get_sysvar::<Clock>().await;
    let epoch = clock_sysvar.epoch;
    let fee = fee_config
        .calculate_epoch_fee(epoch, transfer_amount)
        .unwrap();

    let transfer_logs = transfer_tx.metadata.unwrap().log_messages;
    let transfer_event = transfer_logs
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTransfer::try_from_log(log).ok())
        .unwrap();

    let amount_after_fee = transfer_amount.checked_sub(fee).unwrap();
    assert_eq!(transfer_event.amount, amount_after_fee,);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_canonical_token_maximum_fee_cap(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let fee_basis_points = 1000_u16; // 10% fee rate
    let maximum_fee = 50_u64; // But capped at 50 tokens
    let canonical_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint_with_fee(
            &ctx.solana_wallet,
            &spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            9,
            None,
            None,
        )
        .await;

    // Create Metaplex metadata for the canonical token
    let (canonical_metadata_pda, _) = Metadata::find_pda(&canonical_mint);
    let create_metadata_ix = CreateV1Builder::new()
        .metadata(canonical_metadata_pda)
        .mint(canonical_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Capped Fee Canonical Token".to_string())
        .symbol("CFCT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_metadata_ix]).await.unwrap();

    // Register and deploy
    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    let deploy_remote_canonical_ix =
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            ctx.evm_chain_name.clone(),
            0,
        )?;

    let tx = ctx
        .send_solana_tx(&[deploy_remote_canonical_ix])
        .await
        .unwrap();

    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);
    ctx.relay_to_evm(&call_contract_event.payload).await;

    // Test with large transfer that would exceed maximum fee
    let canonical_token_id = axelar_solana_its::canonical_interchain_token_id(&canonical_mint);

    let user_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &canonical_mint,
        &spl_token_2022::id(),
    );
    let create_user_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &canonical_mint,
        &spl_token_2022::id(),
    );

    let large_amount = 1000_u64; // 10% of 1000 = 100, but capped at 50
    let mint_to_user_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &canonical_mint,
        &user_ata,
        &ctx.solana_wallet,
        &[],
        large_amount,
    )?;

    ctx.send_solana_tx(&[create_user_ata_ix, mint_to_user_ix])
        .await
        .unwrap();

    // Test transfer
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        canonical_token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        large_amount,
        canonical_mint,
        spl_token_2022::id(),
        0,
    )?;

    let transfer_tx = ctx.send_solana_tx(&[transfer_ix]).await.unwrap();

    // Verify maximum fee cap is applied
    let mint_data = ctx
        .solana_chain
        .try_get_account_no_checks(&canonical_mint)
        .await
        .unwrap()
        .unwrap();
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data.data).unwrap();
    let fee_config = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    let clock_sysvar = ctx.solana_chain.get_sysvar::<Clock>().await;
    let epoch = clock_sysvar.epoch;
    let fee = fee_config.calculate_epoch_fee(epoch, large_amount).unwrap();

    // Fee should be capped at maximum_fee (50), not 10% of 1000 (100)
    assert_eq!(fee, maximum_fee);

    let transfer_logs = transfer_tx.metadata.unwrap().log_messages;
    let transfer_event = transfer_logs
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTransfer::try_from_log(log).ok())
        .unwrap();

    let expected_after_fee = large_amount - maximum_fee;
    assert_eq!(transfer_event.amount, expected_after_fee);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_custom_token_with_fee_lock_unlock_fee(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let fee_basis_points = 300_u16; // 3% fee
    let maximum_fee = 1000_u64;
    let token_name = "Custom Fee Token";
    let token_symbol = "CFT";
    let salt = solana_sdk::keccak::hash(b"custom-fee-token-salt").to_bytes();

    // Deploy EVM custom token
    let evm_custom_token = ctx
        .evm_signer
        .deploy_axelar_custom_test_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await?;

    // Create Solana token with fee extension
    let solana_custom_token = ctx
        .solana_chain
        .fixture
        .init_new_mint_with_fee(
            &ctx.solana_wallet,
            &spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            9,
            None,
            None,
        )
        .await;

    // Create Metaplex metadata
    let (metadata_pda, _) = Metadata::find_pda(&solana_custom_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(solana_custom_token, false)
        .authority(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .payer(ctx.solana_wallet)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .is_mutable(false)
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .instruction();

    // Register token metadata
    let register_metadata = axelar_solana_its::instruction::register_token_metadata(
        ctx.solana_wallet,
        solana_custom_token,
        0,
    )?;

    // Send metadata creation first
    ctx.send_solana_tx(&[metadata_ix]).await.unwrap();

    // Then register token metadata
    ctx.send_solana_tx(&[register_metadata]).await.unwrap();

    // Register token metadata on EVM
    ctx.evm_its_contracts
        .interchain_token_service
        .register_token_metadata(evm_custom_token.address(), 0.into())
        .send()
        .await?
        .await?;

    // Register custom token with LockUnlockFee type (specifically for fee handling)
    let token_id = axelar_solana_its::linked_token_id(&ctx.solana_wallet, &salt);
    let register_custom_token_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        salt,
        solana_custom_token,
        axelar_solana_its::state::token_manager::Type::LockUnlockFee,
        spl_token_2022::id(),
        None,
    )?;

    ctx.send_solana_tx(&[register_custom_token_ix])
        .await
        .unwrap();

    // Link token from Solana to EVM
    let link_token_ix = axelar_solana_its::instruction::link_token(
        ctx.solana_wallet,
        salt,
        ctx.evm_chain_name.clone(),
        evm_custom_token.address().as_bytes().to_vec(),
        axelar_solana_its::state::token_manager::Type::LockUnlockFee,
        vec![],
        0,
    )?;

    let tx = ctx.send_solana_tx(&[link_token_ix]).await.unwrap();
    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);

    // Relay to EVM to create token manager
    ctx.relay_to_evm(&call_contract_event.payload).await;

    // Verify token manager was created with correct type
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);

    let data = ctx
        .solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data)?;

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(
        solana_custom_token.as_ref(),
        token_manager.token_address.as_ref()
    );
    assert_eq!(
        token_manager.ty,
        axelar_solana_its::state::token_manager::Type::LockUnlockFee
    );

    // Set up EVM side - mint tokens to the token manager for unlocking
    let token_manager_address = ctx
        .evm_its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .call()
        .await?;

    evm_custom_token
        .mint(token_manager_address, 10000u64.into())
        .send()
        .await?
        .await?;

    evm_custom_token
        .approve(
            ctx.evm_its_contracts.interchain_token_service.address(),
            u64::MAX.into(),
        )
        .send()
        .await?
        .await?;

    // Test outbound transfer (Solana to EVM) with fee handling
    let user_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_custom_token,
        &spl_token_2022::id(),
    );

    let create_user_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &solana_custom_token,
        &spl_token_2022::id(),
    );

    let user_balance = 10000_u64;
    let mint_to_user_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &solana_custom_token,
        &user_ata,
        &ctx.solana_wallet,
        &[],
        user_balance,
    )?;

    ctx.send_solana_tx(&[create_user_ata_ix, mint_to_user_ix])
        .await
        .unwrap();

    // Outbound transfer
    let transfer_amount = 3000_u64;
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        solana_custom_token,
        spl_token_2022::id(),
        0,
    )?;

    let outbound_tx = ctx.send_solana_tx(&[transfer_ix]).await.unwrap();

    // Calculate expected fee for outbound transfer
    let mint_data = ctx
        .solana_chain
        .try_get_account_no_checks(&solana_custom_token)
        .await
        .unwrap()
        .unwrap();
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data.data).unwrap();
    let fee_config = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    let clock_sysvar = ctx.solana_chain.get_sysvar::<Clock>().await;
    let epoch = clock_sysvar.epoch;
    let outbound_fee = fee_config
        .calculate_epoch_fee(epoch, transfer_amount)
        .unwrap();

    // Verify outbound transfer event shows correct amount after fee
    let outbound_logs = outbound_tx.metadata.as_ref().unwrap().log_messages.clone();
    let outbound_event = outbound_logs
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTransfer::try_from_log(log).ok())
        .unwrap();

    let outbound_amount_after_fee = transfer_amount.checked_sub(outbound_fee).unwrap();
    assert_eq!(outbound_event.amount, outbound_amount_after_fee);

    // Relay outbound transfer to EVM
    let call_contract_event = fetch_first_call_contract_event_from_tx(&outbound_tx);
    ctx.relay_to_evm(&call_contract_event.payload).await;

    // Verify EVM received correct amount
    let evm_balance = evm_custom_token
        .balance_of(ctx.evm_signer.wallet.address())
        .call()
        .await?;
    // EVM side should receive the amount after Solana fee deduction
    assert_eq!(evm_balance, outbound_amount_after_fee.into());

    // Test inbound transfer (EVM to Solana) with fee handling
    let inbound_transfer_amount = 1500_u64;

    // Send from EVM to Solana
    ctx.evm_its_contracts
        .interchain_token_service
        .interchain_transfer(
            token_id,
            ctx.solana_chain_name.clone(),
            ctx.solana_wallet.to_bytes().into(),
            inbound_transfer_amount.into(),
            vec![].into(),
            0.into(),
        )
        .send()
        .await?
        .await?;

    // Get the contract call from EVM
    let log = ctx
        .evm_its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await?
        .into_iter()
        .last()
        .ok_or_else(|| anyhow!("no logs found"))?;

    // Relay inbound transfer to Solana
    let inbound_tx = ctx
        .relay_to_solana(
            log.payload.as_ref(),
            Some(solana_custom_token),
            spl_token_2022::id(),
        )
        .await;

    // Calculate expected inbound fee
    let inbound_fee = fee_config
        .calculate_epoch_fee(epoch, inbound_transfer_amount)
        .unwrap();
    let inbound_amount_after_fee = inbound_transfer_amount.checked_sub(inbound_fee).unwrap();

    // Verify user received correct amount after inbound fee
    let user_ata_data = ctx
        .solana_chain
        .try_get_account_no_checks(&user_ata)
        .await
        .unwrap()
        .unwrap();
    let user_account = Account::unpack_from_slice(&user_ata_data.data).unwrap();

    let expected_final_balance = user_balance - transfer_amount + inbound_amount_after_fee;
    assert_eq!(user_account.amount, expected_final_balance);

    // Verify inbound transfer received event
    let inbound_logs = inbound_tx.metadata.as_ref().unwrap().log_messages.clone();
    let received_event = inbound_logs
        .iter()
        .find_map(|log| {
            axelar_solana_its::event::InterchainTransferReceived::try_from_log(log).ok()
        })
        .unwrap();

    assert_eq!(received_event.amount, inbound_amount_after_fee);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_canonical_token_with_fee_uses_lock_unlock_fee(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a canonical token with fee extension
    let fee_basis_points = 500_u16; // 5%
    let maximum_fee = 10000_u64;
    let canonical_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint_with_fee(
            &ctx.solana_wallet,
            &spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            9,
            None,
            None,
        )
        .await;

    // Create Metaplex metadata
    let (canonical_metadata_pda, _) = Metadata::find_pda(&canonical_mint);
    let create_metadata_ix = CreateV1Builder::new()
        .metadata(canonical_metadata_pda)
        .mint(canonical_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Fee Token".to_string())
        .symbol("FT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_metadata_ix]).await.unwrap();

    // Register canonical token
    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    // Check that the token manager uses LockUnlockFee type
    let canonical_token_id = axelar_solana_its::canonical_interchain_token_id(&canonical_mint);
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &canonical_token_id);

    let token_manager_account = ctx
        .solana_chain
        .try_get_account_no_checks(&token_manager_pda)
        .await
        .unwrap()
        .unwrap();

    let token_manager = TokenManager::try_from_slice(&token_manager_account.data).unwrap();

    assert_eq!(
        token_manager.ty,
        axelar_solana_its::state::token_manager::Type::LockUnlockFee,
        "Canonical token with fee extension should use LockUnlockFee token manager"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_canonical_token_without_fee_uses_lock_unlock(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a canonical token without fee extension
    let canonical_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    // Create Metaplex metadata
    let (canonical_metadata_pda, _) = Metadata::find_pda(&canonical_mint);
    let create_metadata_ix = CreateV1Builder::new()
        .metadata(canonical_metadata_pda)
        .mint(canonical_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("No Fee Token".to_string())
        .symbol("NFT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_metadata_ix]).await.unwrap();

    // Register canonical token
    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    // Check that the token manager uses LockUnlock type
    let canonical_token_id = axelar_solana_its::canonical_interchain_token_id(&canonical_mint);
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &canonical_token_id);

    let token_manager_account = ctx
        .solana_chain
        .try_get_account_no_checks(&token_manager_pda)
        .await
        .unwrap()
        .unwrap();

    let token_manager = TokenManager::try_from_slice(&token_manager_account.data).unwrap();

    assert_eq!(
        token_manager.ty,
        axelar_solana_its::state::token_manager::Type::LockUnlock,
        "Canonical token without fee extension should use LockUnlock token manager"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_custom_token_registration_rejects_lock_unlock_with_fee(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a custom token with fee extension
    let fee_basis_points = 300_u16; // 3%
    let maximum_fee = 5000_u64;
    let custom_token = ctx
        .solana_chain
        .fixture
        .init_new_mint_with_fee(
            &ctx.solana_wallet,
            &spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            9,
            None,
            None,
        )
        .await;

    // Create Metaplex metadata
    let (metadata_pda, _) = Metadata::find_pda(&custom_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .mint(custom_token, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Custom Fee Token".to_string())
        .symbol("CFT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[metadata_ix]).await.unwrap();

    // Try to register with LockUnlock (should fail)
    let salt = [1u8; 32];
    let register_custom_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        salt,
        custom_token,
        axelar_solana_its::state::token_manager::Type::LockUnlock,
        spl_token_2022::id(),
        None,
    )?;

    let result = ctx.send_solana_tx(&[register_custom_ix]).await;

    assert!(
        result.is_err(),
        "Expected registration to fail when using LockUnlock with fee extension"
    );

    let error_tx = result.unwrap_err();
    assert_msg_present_in_logs(error_tx, "The mint is not compatible with the type");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_custom_token_registration_rejects_lock_unlock_fee_without_fee(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a custom token without fee extension
    let custom_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    // Create Metaplex metadata
    let (metadata_pda, _) = Metadata::find_pda(&custom_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .mint(custom_token, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Custom No Fee Token".to_string())
        .symbol("CNFT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[metadata_ix]).await.unwrap();

    // Try to register with LockUnlockFee (should fail)
    let salt = [2u8; 32];
    let register_custom_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        salt,
        custom_token,
        axelar_solana_its::state::token_manager::Type::LockUnlockFee,
        spl_token_2022::id(),
        None,
    )?;

    let result = ctx.send_solana_tx(&[register_custom_ix]).await;

    assert!(
        result.is_err(),
        "Expected registration to fail when using LockUnlockFee without fee extension"
    );

    let error_tx = result.unwrap_err();
    assert_msg_present_in_logs(error_tx, "The mint is not compatible with the type");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_custom_token_registration_accepts_lock_unlock_without_fee(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a custom token without fee extension
    let custom_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    // Create Metaplex metadata
    let (metadata_pda, _) = Metadata::find_pda(&custom_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .mint(custom_token, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Valid Custom Token".to_string())
        .symbol("VCT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[metadata_ix]).await.unwrap();

    // Register with LockUnlock (should succeed)
    let salt = [3u8; 32];
    let register_custom_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        salt,
        custom_token,
        axelar_solana_its::state::token_manager::Type::LockUnlock,
        spl_token_2022::id(),
        None,
    )?;

    let result = ctx.send_solana_tx(&[register_custom_ix]).await;

    assert!(
        result.is_ok(),
        "Expected registration to succeed when using LockUnlock without fee extension"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_custom_token_registration_accepts_lock_unlock_fee_with_fee(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a custom token with fee extension
    let fee_basis_points = 250_u16; // 2.5%
    let maximum_fee = 8000_u64;
    let custom_token = ctx
        .solana_chain
        .fixture
        .init_new_mint_with_fee(
            &ctx.solana_wallet,
            &spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            9,
            None,
            None,
        )
        .await;

    // Create Metaplex metadata
    let (metadata_pda, _) = Metadata::find_pda(&custom_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .mint(custom_token, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Valid Fee Token".to_string())
        .symbol("VFT".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[metadata_ix]).await.unwrap();

    // Register with LockUnlockFee (should succeed)
    let salt = [4u8; 32];
    let register_custom_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        salt,
        custom_token,
        axelar_solana_its::state::token_manager::Type::LockUnlockFee,
        spl_token_2022::id(),
        None,
    )?;

    let result = ctx.send_solana_tx(&[register_custom_ix]).await;

    assert!(
        result.is_ok(),
        "Expected registration to succeed when using LockUnlockFee with fee extension"
    );

    Ok(())
}
