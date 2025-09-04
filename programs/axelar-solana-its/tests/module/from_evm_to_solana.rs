use alloy_primitives::{hex, U256};
use anyhow::anyhow;
use axelar_solana_gateway_test_fixtures::assert_msg_present_in_logs;
use borsh::BorshDeserialize;
use solana_program_test::tokio;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::program_pack::Pack as _;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer as _;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use test_context::test_context;

use axelar_message_primitives::{DataPayload, EncodingScheme, SolanaAccountRepr};
use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_its::state::token_manager::{TokenManager, Type as TokenManagerType};
use axelar_solana_memo_program::state::Counter;
use evm_contracts_test_suite::ethers::{signers::Signer, types::Bytes};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;
use evm_contracts_test_suite::evm_contracts_rs::contracts::custom_test_token::CustomTestToken;
use evm_contracts_test_suite::evm_contracts_rs::contracts::interchain_token::InterchainToken;
use evm_contracts_test_suite::ContractMiddleware;
use interchain_token_transfer_gmp::GMPPayload;

use crate::{
    fetch_first_call_contract_event_from_tx, retrieve_evm_log_with_filter, ItsTestContext,
};

async fn custom_token(
    ctx: &mut ItsTestContext,
    token_manager_type: TokenManagerType,
) -> anyhow::Result<([u8; 32], CustomTestToken<ContractMiddleware>, Pubkey)> {
    let token_manager_type: U256 = token_manager_type.into();
    let token_name = "Custom Token";
    let token_symbol = "CT";
    let salt = solana_sdk::keccak::hash(b"custom-token-salt").to_bytes();
    let custom_token = ctx
        .evm_signer
        .deploy_axelar_custom_test_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await?;

    ctx.evm_its_contracts
        .interchain_token_service
        .register_token_metadata(custom_token.address(), 0.into())
        .send()
        .await?
        .await?;

    ctx.evm_its_contracts
        .interchain_token_factory
        .register_custom_token(
            salt,
            custom_token.address(),
            token_manager_type.try_into()?,
            ctx.evm_signer.wallet.address(),
        )
        .send()
        .await?
        .await?;

    let event_filter = ctx
        .evm_its_contracts
        .interchain_token_service
        .interchain_token_id_claimed_filter();

    let token_id = event_filter
        .query()
        .await?
        .first()
        .ok_or_else(|| anyhow!("no token id found"))?
        .token_id;

    let custom_solana_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let register_metadata = axelar_solana_its::instruction::register_token_metadata(
        ctx.solana_wallet,
        custom_solana_token,
        0,
    )?;

    let tx = ctx.send_solana_tx(&[register_metadata]).await.unwrap();
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
        .interchain_token_factory
        .link_token(
            salt,
            ctx.solana_chain_name.clone(),
            custom_solana_token.to_bytes().into(),
            token_manager_type.try_into()?,
            ctx.solana_wallet.to_bytes().into(),
            0_u128.into(),
        )
        .send()
        .await?
        .await?;

    let log: ContractCallFilter = ctx
        .evm_its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no logs found"))?;

    ctx.relay_to_solana(
        log.payload.as_ref(),
        Some(custom_solana_token),
        spl_token_2022::id(),
    )
    .await;
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
        custom_solana_token.as_ref(),
        token_manager.token_address.as_ref()
    );

    Ok((token_id, custom_token, custom_solana_token))
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
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        solana_token,
        token_account,
        ctx.solana_wallet,
        spl_token_2022::id(),
        initial_balance,
    )?;

    ctx.send_solana_tx(&[
        ComputeBudgetInstruction::set_compute_unit_limit(250_000),
        authority_transfer_ix,
        create_ata_ix,
        mint_ix,
    ])
    .await
    .unwrap();

    ctx.test_interchain_transfer(token_id, solana_token, initial_balance, token_account)
        .await;

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_call_contract_with_token(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let memo_instruction =
        axelar_solana_memo_program::instruction::AxelarMemoInstruction::ProcessMemo {
            memo: "🐪🐪🐪🐪".to_owned(),
        };

    let evm_token_address = ctx
        .evm_its_contracts
        .interchain_token_service
        .registered_token_address(ctx.deployed_interchain_token)
        .call()
        .await?;

    let evm_token = InterchainToken::new(
        evm_token_address,
        ctx.evm_its_contracts.interchain_token_service.client(),
    );

    let transfer_amount = 500_000_u64;
    evm_token
        .mint(ctx.evm_signer.wallet.address(), transfer_amount.into())
        .send()
        .await?
        .await?;

    let metadata = Bytes::from(
        [
            0_u32.to_le_bytes().as_slice(), // MetadataVersion.CONTRACT_CALL
            &DataPayload::new(
                &borsh::to_vec(&memo_instruction).unwrap(),
                &[SolanaAccountRepr {
                    pubkey: ctx.counter_pda.to_bytes().into(),
                    is_signer: false,
                    is_writable: true,
                }],
                EncodingScheme::AbiEncoding,
            )
            .encode()?,
        ]
        .concat(),
    );

    ctx.evm_its_contracts
        .interchain_token_service
        .interchain_transfer(
            ctx.deployed_interchain_token,
            ctx.solana_chain_name.clone(),
            axelar_solana_memo_program::id().to_bytes().into(),
            transfer_amount.into(),
            metadata,
            0_u128.into(),
        )
        .send()
        .await?
        .await?
        .unwrap();

    let log =
        retrieve_evm_log_with_filter(ctx.evm_its_contracts.gateway.contract_call_filter()).await;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &ctx.deployed_interchain_token);

    let data = ctx
        .solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data)?;

    let tx = ctx
        .relay_to_solana(
            log.payload.as_ref(),
            Some(token_manager.token_address),
            spl_token_2022::id(),
        )
        .await;

    let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &axelar_solana_memo_program::id(),
        &token_manager.token_address,
        &spl_token_2022::id(),
    );

    let ata_raw_account = ctx
        .solana_chain
        .try_get_account_no_checks(&ata)
        .await?
        .unwrap();

    let ata_account = spl_token_2022::state::Account::unpack_from_slice(&ata_raw_account.data)?;

    assert_eq!(ata_account.mint, token_manager.token_address);
    assert_eq!(ata_account.owner, axelar_solana_memo_program::id());
    assert_eq!(ata_account.amount, transfer_amount);

    assert!(
        tx.find_log("🐪🐪🐪🐪").is_some(),
        "expected memo not found in logs"
    );

    // Verify that the InterchainTransferReceived event contains the correct source address
    let expected_hex = hex::encode(ctx.evm_signer.wallet.address().as_bytes());
    assert_msg_present_in_logs(tx, &format!("payload source address: {expected_hex}"));

    let counter_raw_account = ctx
        .solana_chain
        .try_get_account_no_checks(&ctx.counter_pda)
        .await?
        .unwrap();
    let counter = Counter::try_from_slice(&counter_raw_account.data)?;

    assert_eq!(counter.counter, 1);

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
