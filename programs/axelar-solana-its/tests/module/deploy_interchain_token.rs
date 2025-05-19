use anyhow::anyhow;
use event_utils::Event as _;
use solana_program_test::tokio;
use solana_sdk::program_pack::Pack as _;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use test_context::test_context;

use crate::ItsTestContext;

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_no_minter_and_no_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let salt = solana_sdk::keccak::hash(b"NoMinterNoSupplyToken").0;
    let initial_supply = 0u64;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "No Supply No Minter Token".to_owned(),
        "NSMT".to_owned(),
        9,
        initial_supply,
        None,
    )?;

    let result = ctx.send_solana_tx(&[deploy_local_ix]).await;

    assert!(result.is_err(), "Expected transaction to fail");
    let err = result.unwrap_err();

    let error_logs = err.metadata.unwrap().log_messages;
    let has_invalid_arg_error = error_logs
        .iter()
        .any(|log| log.contains("invalid program argument"));

    assert!(
        has_invalid_arg_error,
        "Expected InvalidArgument error when deploying with no minter and no initial supply"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_minter_but_no_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let salt = solana_sdk::keccak::hash(b"MinterNoSupplyToken").0;
    let initial_supply = 0u64;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Zero Supply Token".to_owned(),
        "ZST".to_owned(),
        9,
        initial_supply,
        Some(ctx.solana_wallet),
    )?;

    let tx = ctx
        .send_solana_tx(&[deploy_local_ix])
        .await
        .expect("InterchainToken deployment failed");

    let deploy_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenDeployed::try_from_log(log).ok())
        .unwrap();

    assert_eq!(
        deploy_event.name, "Zero Supply Token",
        "token name does not match"
    );
    assert_eq!(deploy_event.symbol, "ZST", "token symbol does not match");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let payer_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let token_account_opt = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?;

    if let Some(token_account) = token_account_opt {
        let account = spl_token_2022::state::Account::unpack_from_slice(&token_account.data)?;
        assert_eq!(account.amount, 0, "Initial supply should be zero");
    }

    let create_token_account_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.solana_wallet,
            &ctx.solana_wallet,
            &interchain_token_pda,
            &spl_token_2022::id(),
        );

    ctx.send_solana_tx(&[create_token_account_ix])
        .await
        .expect("Failed to create token account");

    let mint_amount = 500u64;
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        interchain_token_pda,
        payer_ata,
        ctx.solana_wallet,
        spl_token_2022::id(),
        mint_amount,
    )?;

    ctx.send_solana_tx(&[mint_ix])
        .await
        .expect("Minting tokens failed");

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?
        .ok_or_else(|| anyhow!("token account not found"))?
        .data;

    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;

    assert_eq!(
        account.amount, mint_amount,
        "Minted amount doesn't match expected amount"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_large_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let salt = solana_sdk::keccak::hash(b"LargeSupplyTestToken").0;
    let initial_supply = u64::MAX;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Large Supply Token".to_owned(),
        "LST".to_owned(),
        9,
        initial_supply,
        Some(ctx.solana_wallet),
    )?;

    let tx = ctx
        .send_solana_tx(&[deploy_local_ix])
        .await
        .expect("InterchainToken deployment failed");

    let deploy_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenDeployed::try_from_log(log).ok())
        .unwrap();

    assert_eq!(
        deploy_event.name, "Large Supply Token",
        "token name does not match"
    );
    assert_eq!(deploy_event.symbol, "LST", "token symbol does not match");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let payer_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?
        .ok_or_else(|| anyhow!("token account not found"))?
        .data;

    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;

    assert_eq!(
        account.amount, initial_supply,
        "Initial supply doesn't match expected amount"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_no_minter_but_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let salt = solana_sdk::keccak::hash(b"NoMinterWithSupplyToken").0;
    let initial_supply = 1000u64;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Fixed Supply Token".to_owned(),
        "FST".to_owned(),
        9,
        initial_supply,
        None,
    )?;

    let tx = ctx
        .send_solana_tx(&[deploy_local_ix])
        .await
        .expect("InterchainToken deployment failed");

    let deploy_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenDeployed::try_from_log(log).ok())
        .unwrap();

    assert_eq!(
        deploy_event.name, "Fixed Supply Token",
        "token name does not match"
    );
    assert_eq!(deploy_event.symbol, "FST", "token symbol does not match");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let payer_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?
        .ok_or_else(|| anyhow!("token account not found"))?
        .data;

    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;

    assert_eq!(
        account.amount, initial_supply,
        "Initial supply doesn't match expected amount"
    );

    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        interchain_token_pda,
        payer_ata,
        ctx.solana_wallet,
        spl_token_2022::id(),
        100,
    )?;

    let result = ctx.send_solana_tx(&[mint_ix]).await;

    assert!(
        result.is_err(),
        "Expected minting to fail for fixed supply token"
    );

    Ok(())
}
