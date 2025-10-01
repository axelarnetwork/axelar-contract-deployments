use axelar_solana_gateway_test_fixtures::assert_msg_present_in_logs;
use event_utils::Event;
use evm_contracts_test_suite::ethers::signers::Signer;
use solana_program_test::tokio;
use test_context::test_context;

use crate::ItsTestContext;

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_local_deployment_rejects_long_name(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let salt = [1u8; 32];
    let long_name = "A".repeat(33);
    let valid_symbol = "VALID";

    let deploy_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        long_name,
        valid_symbol.to_string(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    let result = ctx.send_solana_tx(&[deploy_ix]).await;
    assert!(result.is_err(),);
    let error_tx = result.unwrap_err();
    assert_msg_present_in_logs(error_tx, "Name and/or symbol length too long");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_local_deployment_rejects_long_symbol(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let salt = [2u8; 32];
    let valid_name = "Valid Name";
    let long_symbol = "ABCDEFGHIJK";

    let deploy_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        valid_name.to_string(),
        long_symbol.to_string(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    let result = ctx.send_solana_tx(&[deploy_ix]).await;

    assert!(result.is_err(),);
    let error_tx = result.unwrap_err();
    assert_msg_present_in_logs(error_tx, "Name and/or symbol length too long");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_local_deployment_rejects_long_name_and_symbol(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let salt = [3u8; 32];
    let long_name = "This is a very long name that exceeds thirty-two characters";
    let long_symbol = "VERYLONGSYM";

    let deploy_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        long_name.to_string(),
        long_symbol.to_string(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    let result = ctx.send_solana_tx(&[deploy_ix]).await;

    assert!(result.is_err(),);
    let error_tx = result.unwrap_err();
    assert_msg_present_in_logs(error_tx, "Name and/or symbol length too long");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_local_deployment_succeeds_with_valid_lengths(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let salt = [4u8; 32];
    let valid_name = "Valid Token Name";
    let valid_symbol = "VALID";

    let deploy_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        valid_name.to_string(),
        valid_symbol.to_string(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    let result = ctx.send_solana_tx(&[deploy_ix]).await;

    assert!(result.is_ok());

    let tx = result.unwrap();
    let deploy_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::events::InterchainTokenDeployed::try_from_log(log).ok())
        .unwrap();

    assert_eq!(deploy_event.name, valid_name);
    assert_eq!(deploy_event.symbol, valid_symbol);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_local_deployment_succeeds_with_max_lengths(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let salt = [5u8; 32];
    let max_name = "A".repeat(32);
    let max_symbol = "B".repeat(10);

    let deploy_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        max_name.clone(),
        max_symbol.clone(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    let result = ctx.send_solana_tx(&[deploy_ix]).await;

    assert!(result.is_ok());

    let tx = result.unwrap();
    let deploy_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::events::InterchainTokenDeployed::try_from_log(log).ok())
        .unwrap();

    assert_eq!(deploy_event.name, max_name);
    assert_eq!(deploy_event.symbol, max_symbol);

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_incoming_deployment_truncates_long_name(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;

    let salt = [1u8; 32];
    let long_name = "This is an extremely long token name that exceeds thirty-two characters and should be truncated"; // 94 characters
    let valid_symbol = "SYMBOL";

    // Deploy token on EVM with long metadata
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_interchain_token(
            salt,
            long_name.to_string(),
            valid_symbol.to_string(),
            18,       // EVM decimals
            0.into(), // initial supply
            ctx.evm_signer.wallet.address(),
        )
        .send()
        .await?
        .await?;

    // Deploy remotely to Solana
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_remote_interchain_token(salt, ctx.solana_chain_name.clone(), 0.into())
        .send()
        .await?
        .await?;

    // Capture the contract call and relay it to Solana
    let log: ContractCallFilter = ctx
        .evm_its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await?
        .into_iter()
        .last()
        .expect("Should have contract call");

    // Relay the deployment message to Solana
    let result = ctx
        .relay_to_solana(log.payload.as_ref(), None, spl_token_2022::id())
        .await;

    // Should succeed (not fail like local deployment)
    assert!(
        result.metadata.is_some(),
        "Expected incoming deployment to succeed with metadata truncation"
    );

    // Verify the deployment event shows truncated name (32 characters)
    let logs = result.metadata.unwrap().log_messages;
    let deploy_event = logs
        .iter()
        .find_map(|log| axelar_solana_its::events::InterchainTokenDeployed::try_from_log(log).ok())
        .expect("Should emit InterchainTokenDeployed event");

    assert_eq!(
        deploy_event.name.len(),
        32,
        "Name should be truncated to 32 characters"
    );
    assert_eq!(&deploy_event.name, "This is an extremely long token ");
    assert_eq!(
        deploy_event.symbol, valid_symbol,
        "Symbol should remain unchanged"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_incoming_deployment_truncates_long_symbol(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;

    let salt = [2u8; 32];
    let valid_name = "Valid Token Name";
    let long_symbol = "VERYLONGSYMBOL";

    // Deploy token on EVM with long symbol
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_interchain_token(
            salt,
            valid_name.to_string(),
            long_symbol.to_string(),
            18,
            0.into(),
            ctx.evm_signer.wallet.address(),
        )
        .send()
        .await?
        .await?;

    // Deploy remotely to Solana
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_remote_interchain_token(salt, ctx.solana_chain_name.clone(), 0.into())
        .send()
        .await?
        .await?;

    // Capture the contract call and relay it to Solana
    let log: ContractCallFilter = ctx
        .evm_its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await?
        .into_iter()
        .last()
        .expect("Should have contract call");

    // Relay the deployment message to Solana
    let result = ctx
        .relay_to_solana(log.payload.as_ref(), None, spl_token_2022::id())
        .await;

    // Should succeed
    assert!(result.metadata.is_some());

    // Verify the deployment event shows truncated symbol (10 characters)
    let logs = result.metadata.unwrap().log_messages;
    let deploy_event = logs
        .iter()
        .find_map(|log| axelar_solana_its::events::InterchainTokenDeployed::try_from_log(log).ok())
        .expect("Should emit InterchainTokenDeployed event");

    assert_eq!(deploy_event.name, valid_name);
    assert_eq!(deploy_event.symbol.len(), 10);
    assert_eq!(&deploy_event.symbol, "VERYLONGSY");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_incoming_deployment_truncates_long_name_and_symbol(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;

    let salt = [3u8; 32];
    let long_name = "This is another extremely long token name that will definitely exceed the maximum allowed length of thirty-two characters";
    let long_symbol = "ANOTHERLONGSYMBOL";

    // Deploy token on EVM with both long name and symbol
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_interchain_token(
            salt,
            long_name.to_string(),
            long_symbol.to_string(),
            18,
            0.into(),
            ctx.evm_signer.wallet.address(),
        )
        .send()
        .await?
        .await?;

    // Deploy remotely to Solana
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_remote_interchain_token(salt, ctx.solana_chain_name.clone(), 0.into())
        .send()
        .await?
        .await?;

    // Capture the contract call and relay it to Solana
    let log: ContractCallFilter = ctx
        .evm_its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await?
        .into_iter()
        .last()
        .expect("Should have contract call");

    // Relay the deployment message to Solana
    let result = ctx
        .relay_to_solana(log.payload.as_ref(), None, spl_token_2022::id())
        .await;

    assert!(result.metadata.is_some());

    // Verify the deployment event shows both name and symbol truncated
    let logs = result.metadata.unwrap().log_messages;
    let deploy_event = logs
        .iter()
        .find_map(|log| axelar_solana_its::events::InterchainTokenDeployed::try_from_log(log).ok())
        .expect("Should emit InterchainTokenDeployed event");

    assert_eq!(deploy_event.name.len(), 32);
    assert_eq!(&deploy_event.name, "This is another extremely long t");
    assert_eq!(deploy_event.symbol.len(), 10);
    assert_eq!(&deploy_event.symbol, "ANOTHERLON");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_incoming_deployment_succeeds_with_valid_lengths(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;

    let salt = [4u8; 32];
    let valid_name = "Valid Incoming Token";
    let valid_symbol = "VIT";

    // Deploy token on EVM with valid lengths
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_interchain_token(
            salt,
            valid_name.to_string(),
            valid_symbol.to_string(),
            18,
            0.into(),
            ctx.evm_signer.wallet.address(),
        )
        .send()
        .await?
        .await?;

    // Deploy remotely to Solana
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_remote_interchain_token(
            salt,
            ctx.solana_chain_name.clone(),
            0.into(), // gas value
        )
        .send()
        .await?
        .await?;

    // Capture the contract call and relay it to Solana
    let log: ContractCallFilter = ctx
        .evm_its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await?
        .into_iter()
        .last()
        .expect("Should have contract call");

    // Relay the deployment message to Solana
    let result = ctx
        .relay_to_solana(log.payload.as_ref(), None, spl_token_2022::id())
        .await;

    assert!(result.metadata.is_some());

    // Verify the deployment event shows original name and symbol
    let logs = result.metadata.unwrap().log_messages;
    let deploy_event = logs
        .iter()
        .find_map(|log| axelar_solana_its::events::InterchainTokenDeployed::try_from_log(log).ok())
        .expect("Should emit InterchainTokenDeployed event");

    assert_eq!(deploy_event.name, valid_name);
    assert_eq!(deploy_event.symbol, valid_symbol);

    Ok(())
}
