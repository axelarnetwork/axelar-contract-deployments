use borsh::BorshDeserialize;
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer as _;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use test_context::test_context;

use axelar_solana_its::state::token_manager::{TokenManager, Type as TokenManagerType};
use evm_contracts_test_suite::evm_contracts_rs::contracts::{
    custom_test_token::CustomTestToken, interchain_token::InterchainToken,
};
use evm_contracts_test_suite::ContractMiddleware;
use interchain_token_transfer_gmp::GMPPayload;

use crate::{
    fetch_first_call_contract_event_from_tx, retrieve_evm_log_with_filter, ItsTestContext,
};

async fn custom_token(
    ctx: &mut ItsTestContext,
    token_manager_type: TokenManagerType,
) -> anyhow::Result<([u8; 32], CustomTestToken<ContractMiddleware>, Pubkey)> {
    let token_name = "Custom Token";
    let token_symbol = "CT";
    let salt = solana_sdk::keccak::hash(b"custom-token-salt").to_bytes();
    let custom_token = ctx
        .evm_signer
        .deploy_axelar_custom_test_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await?;

    let custom_solana_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let (metadata_pda, _) = Metadata::find_pda(&custom_solana_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(custom_solana_token, false)
        .authority(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .payer(ctx.solana_wallet)
        .is_mutable(false)
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    let register_metadata = axelar_solana_its::instruction::register_token_metadata(
        ctx.solana_wallet,
        custom_solana_token,
        spl_token_2022::id(),
        0,
        axelar_solana_gas_service::id(),
        ctx.solana_gas_utils.config_pda,
    )?;

    let tx = ctx
        .send_solana_tx(&[metadata_ix, register_metadata])
        .await
        .unwrap();
    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);

    let GMPPayload::RegisterTokenMetadata(register_message) =
        GMPPayload::decode(&call_contract_event.payload)?
    else {
        panic!("wrong message");
    };

    assert_eq!(
        register_message.token_address.as_ref(),
        custom_solana_token.as_ref()
    );
    assert_eq!(register_message.decimals, 9);

    ctx.evm_its_contracts
        .interchain_token_service
        .register_token_metadata(custom_token.address(), 0.into())
        .send()
        .await?
        .await?;

    let token_id = axelar_solana_its::linked_token_id(&ctx.solana_wallet, &salt);
    let register_custom_token_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        salt,
        custom_solana_token,
        token_manager_type,
        spl_token_2022::id(),
        None,
    )?;

    ctx.send_solana_tx(&[register_custom_token_ix])
        .await
        .unwrap();

    let link_token_ix = axelar_solana_its::instruction::link_token(
        ctx.solana_wallet,
        salt,
        ctx.evm_chain_name.clone(),
        custom_token.address().as_bytes().to_vec(),
        token_manager_type,
        vec![],
        0,
        axelar_solana_gas_service::id(),
        ctx.solana_gas_utils.config_pda,
    )?;

    let tx = ctx.send_solana_tx(&[link_token_ix]).await.unwrap();
    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);
    let message = if let GMPPayload::SendToHub(inner) =
        GMPPayload::decode(&call_contract_event.payload)?
    {
        let GMPPayload::LinkToken(link_token) = GMPPayload::decode(inner.payload.as_ref())? else {
            panic!("wrong message");
        };
        link_token
    } else {
        panic!("wrong message");
    };

    assert_eq!(
        message.source_token_address.as_ref(),
        custom_solana_token.as_ref()
    );
    assert_eq!(
        message.destination_token_address.as_ref(),
        custom_token.address().as_bytes()
    );
    assert_eq!(token_id, message.token_id);

    ctx.relay_to_evm(&call_contract_event.payload).await;
    let log = retrieve_evm_log_with_filter(
        ctx.evm_its_contracts
            .interchain_token_service
            .token_manager_deployed_filter(),
    )
    .await;

    assert_eq!(log.token_id, token_id);
    assert_eq!(log.token_manager_type, token_manager_type as u8);

    let (its_root_pda, _) =
        axelar_solana_its::find_its_root_pda(&ctx.solana_chain.gateway_root_pda);
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
        custom_solana_token.as_ref(),
        token_manager.token_address.as_ref()
    );

    Ok((token_id, custom_token, custom_solana_token))
}

async fn canonical_token(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<([u8; 32], InterchainToken<ContractMiddleware>, Pubkey)> {
    let token_name = "Canonical Token";
    let token_symbol = "CT";

    let canonical_solana_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let (metadata_pda, _) = Metadata::find_pda(&canonical_solana_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(canonical_solana_token, false)
        .authority(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .payer(ctx.solana_wallet)
        .is_mutable(false)
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    ctx.send_solana_tx(&[metadata_ix]).await.unwrap();

    let register_token = axelar_solana_its::instruction::register_canonical_interchain_token(
        ctx.solana_wallet,
        canonical_solana_token,
        spl_token_2022::id(),
    )?;

    let tx = ctx.send_solana_tx(&[register_token]).await.unwrap();

    let token_id: [u8; 32] = tx
        .metadata
        .as_ref()
        .unwrap()
        .return_data
        .as_ref()
        .unwrap()
        .data
        .as_slice()
        .try_into()
        .unwrap();

    let expected_token_id =
        axelar_solana_its::canonical_interchain_token_id(&canonical_solana_token);

    assert_eq!(expected_token_id, token_id,);

    let deploy_remote_canonical_token_ix =
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_solana_token,
            ctx.evm_chain_name.clone(),
            0,
            axelar_solana_gas_service::id(),
            ctx.solana_gas_utils.config_pda,
        )?;

    let tx = ctx
        .send_solana_tx(&[deploy_remote_canonical_token_ix])
        .await
        .unwrap();

    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);
    let message =
        if let GMPPayload::SendToHub(inner) = GMPPayload::decode(&call_contract_event.payload)? {
            let GMPPayload::DeployInterchainToken(deploy_token) =
                GMPPayload::decode(inner.payload.as_ref())?
            else {
                panic!("wrong message");
            };
            deploy_token
        } else {
            panic!("wrong message");
        };

    assert_eq!(message.token_id, expected_token_id);
    assert_eq!(message.name.trim_end_matches('\0'), token_name);
    assert_eq!(message.symbol.trim_end_matches('\0'), token_symbol);
    assert_eq!(message.decimals, 9);

    ctx.relay_to_evm(&call_contract_event.payload).await;
    let log = retrieve_evm_log_with_filter(
        ctx.evm_its_contracts
            .interchain_token_service
            .interchain_token_deployed_filter(),
    )
    .await;

    assert_eq!(log.token_id, token_id);
    assert_eq!(log.symbol, token_symbol);
    assert_eq!(log.name, token_name);
    assert_eq!(log.decimals, 9);

    let (its_root_pda, _) =
        axelar_solana_its::find_its_root_pda(&ctx.solana_chain.gateway_root_pda);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);

    let evm_token_address = ctx
        .evm_its_contracts
        .interchain_token_service
        .registered_token_address(token_id)
        .call()
        .await?;

    let evm_token = InterchainToken::new(
        evm_token_address,
        ctx.evm_its_contracts.interchain_token_service.client(),
    );

    let data = ctx
        .solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data)?;

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(
        canonical_solana_token.as_ref(),
        token_manager.token_address.as_ref()
    );

    Ok((token_id, evm_token, canonical_solana_token))
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_custom_token_lock_unlock_link_transfer(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (token_id, evm_token, solana_token) =
        custom_token(ctx, TokenManagerType::LockUnlock).await?;

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let create_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let initial_balance = 300;
    let mint_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &solana_token,
        &token_account,
        &ctx.solana_wallet,
        &[],
        initial_balance,
    )?;

    ctx.send_solana_tx(&[create_ata_ix, mint_ix]).await.unwrap();

    // Mint some tokens to the token manager so it can unlock
    let token_manager = ctx
        .evm_its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .call()
        .await?;

    evm_token
        .mint(token_manager, 900.into())
        .send()
        .await?
        .await?;

    evm_token
        .approve(
            ctx.evm_its_contracts.interchain_token_service.address(),
            u64::MAX.into(),
        )
        .send()
        .await?
        .await?;

    ctx.test_interchain_transfer(token_id, solana_token, initial_balance, token_account)
        .await;

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_custom_token_mint_burn_link_transfer(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let (token_id, evm_token, solana_token) = custom_token(ctx, TokenManagerType::MintBurn).await?;

    let authority_transfer_ix =
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            ctx.solana_wallet,
            token_id,
            solana_token,
            spl_token_2022::id(),
        )?;

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let create_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let initial_balance = 300;
    // As the mint authority was handed over, we need to mint through ITS.
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        solana_token,
        token_account,
        ctx.solana_wallet,
        spl_token_2022::id(),
        initial_balance,
    )?;

    ctx.send_solana_tx(&[authority_transfer_ix, create_ata_ix, mint_ix])
        .await
        .unwrap();

    let token_manager = ctx
        .evm_its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .call()
        .await?;

    evm_token
        .transfer_mintership(token_manager)
        .send()
        .await?
        .await?;

    ctx.test_interchain_transfer(token_id, solana_token, initial_balance, token_account)
        .await;

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_canonical_token_lock_unlock_transfer(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let (token_id, _evm_token, solana_token) = canonical_token(ctx).await?;

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let create_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let initial_balance = 300;
    let mint_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &solana_token,
        &token_account,
        &ctx.solana_wallet,
        &[],
        initial_balance,
    )?;

    ctx.send_solana_tx(&[create_ata_ix, mint_ix]).await.unwrap();
    ctx.test_interchain_transfer(token_id, solana_token, initial_balance, token_account)
        .await;

    Ok(())
}

#[test_context(ItsTestContext)]
#[should_panic]
#[tokio::test]
async fn fail_when_chain_not_trusted(ctx: &mut ItsTestContext) {
    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instruction::remove_trusted_chain(
                ctx.solana_chain.upgrade_authority.pubkey(),
                ctx.evm_chain_name.clone(),
            )
            .unwrap()],
            &[
                ctx.solana_chain.upgrade_authority.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let _ = custom_token(ctx, TokenManagerType::MintBurn).await.unwrap();
}
