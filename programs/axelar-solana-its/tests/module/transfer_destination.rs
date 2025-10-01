use event_utils::Event;
use interchain_token_transfer_gmp::{GMPPayload, InterchainTransfer};
use solana_program_test::tokio;
use solana_sdk::program_pack::Pack as _;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer as _;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::instruction::initialize_account3;
use test_context::test_context;

use crate::ItsTestContext;
use axelar_solana_its::state::token_manager::Type as TokenManagerType;

/// Helper function to create a custom mint for testing
async fn setup_custom_mint_and_token_manager(
    ctx: &mut ItsTestContext,
    token_manager_type: TokenManagerType,
) -> anyhow::Result<([u8; 32], Pubkey)> {
    let salt = solana_sdk::keccak::hash(b"wallet-as-token-account-test").to_bytes();

    let custom_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let token_id = axelar_solana_its::linked_token_id(&ctx.solana_wallet, &salt);
    let register_custom_token_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        custom_mint,
        token_manager_type,
        spl_token_2022::id(),
        None,
    )?;

    ctx.send_solana_tx(&[register_custom_token_ix])
        .await
        .unwrap();

    Ok((token_id, custom_mint))
}

/// Creates a token account
async fn create_direct_token_account(
    ctx: &mut ItsTestContext,
    mint: Pubkey,
    owner: Pubkey,
) -> anyhow::Result<Pubkey> {
    let token_account_keypair = Keypair::new();
    let token_account = token_account_keypair.pubkey();

    let rent_exempt_balance = ctx
        .solana_chain
        .fixture
        .get_rent(spl_token_2022::state::Account::LEN)
        .await;

    #[allow(clippy::disallowed_methods)]
    let create_account_ix = solana_sdk::system_instruction::create_account(
        &ctx.solana_wallet,
        &token_account,
        rent_exempt_balance,
        spl_token_2022::state::Account::LEN as u64,
        &spl_token_2022::id(),
    );

    let init_account_ix =
        initialize_account3(&spl_token_2022::id(), &token_account, &mint, &owner)?;

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[create_account_ix, init_account_ix],
            &[
                ctx.solana_chain.fixture.payer.insecure_clone(),
                token_account_keypair.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    Ok(token_account)
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_inbound_transfer_using_token_account_mint_burn(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (token_id, custom_mint) =
        setup_custom_mint_and_token_manager(ctx, TokenManagerType::MintBurn).await?;

    let authority_transfer_ix =
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            ctx.solana_wallet,
            ctx.solana_wallet,
            token_id,
            custom_mint,
            spl_token_2022::id(),
        )?;
    ctx.send_solana_tx(&[authority_transfer_ix]).await.unwrap();

    let token_account = create_direct_token_account(ctx, custom_mint, ctx.solana_wallet).await?;

    let transfer_amount = 300u64;
    let interchain_transfer = InterchainTransfer {
        selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into().unwrap(),
        token_id: token_id.into(),
        source_address: b"0x1234567890123456789012345678901234567890"
            .to_vec()
            .into(),
        destination_address: token_account.to_bytes().into(),
        amount: alloy_primitives::U256::from(transfer_amount),
        data: vec![].into(),
    };

    let payload = GMPPayload::SendToHub(interchain_token_transfer_gmp::SendToHub {
        selector: interchain_token_transfer_gmp::SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .unwrap(),
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(interchain_transfer)
            .encode()
            .into(),
    });

    let tx = ctx
        .relay_to_solana(&payload.encode(), Some(custom_mint), spl_token_2022::id())
        .await;

    let logs = tx.metadata.unwrap().log_messages;
    let transfer_received_event = logs
        .iter()
        .find_map(|log| {
            axelar_solana_its::events::InterchainTransferReceived::try_from_log(log).ok()
        })
        .expect("InterchainTransferReceived event should be present");

    assert_eq!(transfer_received_event.amount, transfer_amount);
    assert_eq!(transfer_received_event.token_id, token_id);
    assert_eq!(transfer_received_event.destination_address, token_account);

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&token_account)
        .await
        .unwrap()
        .unwrap()
        .data;
    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;
    assert_eq!(account.amount, transfer_amount);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_inbound_transfer_using_token_account_lock_unlock(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (token_id, custom_mint) =
        setup_custom_mint_and_token_manager(ctx, TokenManagerType::LockUnlock).await?;

    let token_account = create_direct_token_account(ctx, custom_mint, ctx.solana_wallet).await?;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &custom_mint,
        &spl_token_2022::id(),
    );

    let mint_amount = 1000;
    let mint_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &custom_mint,
        &token_manager_ata,
        &ctx.solana_wallet,
        &[],
        mint_amount,
    )?;
    ctx.send_solana_tx(&[mint_ix]).await.unwrap();

    let transfer_amount = 300u64;
    let interchain_transfer = InterchainTransfer {
        selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into().unwrap(),
        token_id: token_id.into(),
        source_address: b"0x1234567890123456789012345678901234567890"
            .to_vec()
            .into(),
        destination_address: token_account.to_bytes().into(),
        amount: alloy_primitives::U256::from(transfer_amount),
        data: vec![].into(),
    };

    let payload = GMPPayload::SendToHub(interchain_token_transfer_gmp::SendToHub {
        selector: interchain_token_transfer_gmp::SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .unwrap(),
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(interchain_transfer)
            .encode()
            .into(),
    });

    let tx = ctx
        .relay_to_solana(&payload.encode(), Some(custom_mint), spl_token_2022::id())
        .await;

    let logs = tx.metadata.unwrap().log_messages;
    let transfer_received_event = logs
        .iter()
        .find_map(|log| {
            axelar_solana_its::events::InterchainTransferReceived::try_from_log(log).ok()
        })
        .expect("InterchainTransferReceived event should be present");

    assert_eq!(transfer_received_event.amount, transfer_amount);
    assert_eq!(transfer_received_event.token_id, token_id);
    assert_eq!(transfer_received_event.destination_address, token_account);

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&token_account)
        .await
        .unwrap()
        .unwrap()
        .data;
    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;
    assert_eq!(account.amount, transfer_amount);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_inbound_transfer_using_wallet_mint_burn(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (token_id, custom_mint) =
        setup_custom_mint_and_token_manager(ctx, TokenManagerType::MintBurn).await?;

    let authority_transfer_ix =
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            ctx.solana_wallet,
            ctx.solana_wallet,
            token_id,
            custom_mint,
            spl_token_2022::id(),
        )?;
    ctx.send_solana_tx(&[authority_transfer_ix]).await.unwrap();

    let transfer_amount = 300u64;
    let interchain_transfer = InterchainTransfer {
        selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into().unwrap(),
        token_id: token_id.into(),
        source_address: b"0x1234567890123456789012345678901234567890"
            .to_vec()
            .into(),
        destination_address: ctx.solana_wallet.to_bytes().to_vec().into(),
        amount: alloy_primitives::U256::from(transfer_amount),
        data: vec![].into(),
    };

    let payload = GMPPayload::SendToHub(interchain_token_transfer_gmp::SendToHub {
        selector: interchain_token_transfer_gmp::SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .unwrap(),
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(interchain_transfer)
            .encode()
            .into(),
    });

    let tx = ctx
        .relay_to_solana(&payload.encode(), Some(custom_mint), spl_token_2022::id())
        .await;

    let logs = tx.metadata.unwrap().log_messages;
    let transfer_received_event = logs
        .iter()
        .find_map(|log| {
            axelar_solana_its::events::InterchainTransferReceived::try_from_log(log).ok()
        })
        .expect("InterchainTransferReceived event should be present");

    assert_eq!(transfer_received_event.amount, transfer_amount);
    assert_eq!(transfer_received_event.token_id, token_id);
    assert_eq!(
        transfer_received_event.destination_address,
        ctx.solana_wallet
    );

    let wallet_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &custom_mint,
        &spl_token_2022::id(),
    );
    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&wallet_ata)
        .await
        .unwrap()
        .unwrap()
        .data;
    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;
    assert_eq!(account.amount, transfer_amount);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_inbound_transfer_using_wallet_lock_unlock(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (token_id, custom_mint) =
        setup_custom_mint_and_token_manager(ctx, TokenManagerType::LockUnlock).await?;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &custom_mint,
        &spl_token_2022::id(),
    );

    let mint_amount = 1000;
    let mint_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &custom_mint,
        &token_manager_ata,
        &ctx.solana_wallet,
        &[],
        mint_amount,
    )?;
    ctx.send_solana_tx(&[mint_ix]).await.unwrap();

    let transfer_amount = 300u64;
    let interchain_transfer = InterchainTransfer {
        selector: InterchainTransfer::MESSAGE_TYPE_ID.try_into().unwrap(),
        token_id: token_id.into(),
        source_address: b"0x1234567890123456789012345678901234567890"
            .to_vec()
            .into(),
        destination_address: ctx.solana_wallet.to_bytes().into(),
        amount: alloy_primitives::U256::from(transfer_amount),
        data: vec![].into(),
    };

    let payload = GMPPayload::SendToHub(interchain_token_transfer_gmp::SendToHub {
        selector: interchain_token_transfer_gmp::SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .unwrap(),
        destination_chain: ctx.solana_chain_name.clone(),
        payload: GMPPayload::InterchainTransfer(interchain_transfer)
            .encode()
            .into(),
    });

    let tx = ctx
        .relay_to_solana(&payload.encode(), Some(custom_mint), spl_token_2022::id())
        .await;

    let logs = tx.metadata.unwrap().log_messages;
    let transfer_received_event = logs
        .iter()
        .find_map(|log| {
            axelar_solana_its::events::InterchainTransferReceived::try_from_log(log).ok()
        })
        .expect("InterchainTransferReceived event should be present");

    assert_eq!(transfer_received_event.amount, transfer_amount);
    assert_eq!(transfer_received_event.token_id, token_id);
    assert_eq!(
        transfer_received_event.destination_address,
        ctx.solana_wallet
    );

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &custom_mint,
        &spl_token_2022::id(),
    );
    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&token_account)
        .await
        .unwrap()
        .unwrap()
        .data;
    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;
    assert_eq!(account.amount, transfer_amount);

    Ok(())
}
