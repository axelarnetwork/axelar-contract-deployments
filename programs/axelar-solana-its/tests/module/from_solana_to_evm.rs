use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_its::event::InterchainTransfer;
use borsh::BorshDeserialize;
use event_utils::Event;
use evm_contracts_test_suite::ethers::signers::Signer;
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program_test::tokio;
use solana_sdk::program_pack::Pack as _;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer as _;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use test_context::test_context;

use axelar_solana_gateway_test_fixtures::assert_msg_present_in_logs;
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
        0,
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

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
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

    let initial_balance = 300;
    // As the mint authority was handed over, we need to mint through ITS.
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        ctx.solana_wallet,
        token_id,
        solana_token,
        ctx.solana_wallet,
        ctx.solana_wallet,
        spl_token_2022::id(),
        initial_balance,
    )?;

    ctx.send_solana_tx(&[authority_transfer_ix, mint_ix])
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

#[test_context(ItsTestContext)]
#[tokio::test]
async fn transfer_fails_with_wrong_gas_service(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
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
    let mut transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        initial_balance,
        solana_token,
        spl_token_2022::id(),
        1000, // gas_value needs to be greater than 0 for pay_gas to be called
    )
    .unwrap();
    transfer_ix.accounts[9].pubkey = Pubkey::new_unique(); // invalid gas service

    assert!(ctx
        .send_solana_tx(&[transfer_ix])
        .await
        .unwrap_err()
        .find_log("An account required by the instruction is missing")
        .is_some());

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_lock_unlock_transfer_fails_with_token_manager_as_authority(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (token_id, _, solana_token) = custom_token(ctx, TokenManagerType::LockUnlock).await?;

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let token_manager_pda = axelar_solana_its::find_token_manager_pda(
        &axelar_solana_its::find_its_root_pda().0,
        &token_id,
    )
    .0;
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &solana_token,
        &spl_token_2022::id(),
    );

    // Pretend the TokenManager has tokens locked already
    let initial_balance = 300;
    let mint_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &solana_token,
        &token_manager_ata,
        &ctx.solana_wallet,
        &[&ctx.solana_wallet],
        initial_balance,
    )?;

    ctx.send_solana_tx(&[mint_ix]).await.unwrap();

    // Try to transfer from the TokenManager to payer. This should fail after the fix
    let mut transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_chain.fixture.payer.pubkey(),
        ctx.solana_wallet,
        token_id,
        ctx.solana_chain_name.clone(),
        token_account.to_bytes().to_vec(),
        initial_balance,
        solana_token,
        spl_token_2022::id(),
        0,
    )
    .unwrap();
    transfer_ix.accounts[2].pubkey = token_manager_ata;

    assert!(ctx
        .send_solana_tx(&[transfer_ix])
        .await
        .unwrap_err()
        .find_log("Error: owner does not match")
        .is_some());

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_mint_burn_from_interchain_transfer_with_approval(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Initialize a new mint and register as custom token with MintBurnFrom type
    let token_name = "Test Token";
    let token_symbol = "TT";
    let salt = solana_sdk::keccak::hash(b"test-token-salt").to_bytes();
    let custom_token = ctx
        .evm_signer
        .deploy_axelar_custom_test_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await?;

    let solana_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    // Create metadata for the token
    let (metadata_pda, _) = Metadata::find_pda(&solana_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(solana_token, false)
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
        solana_token,
        0,
    )?;

    let _tx = ctx
        .send_solana_tx(&[metadata_ix, register_metadata])
        .await
        .unwrap();

    ctx.evm_its_contracts
        .interchain_token_service
        .register_token_metadata(custom_token.address(), 0.into())
        .send()
        .await?
        .await?;

    // Register the custom token with MintBurnFrom type
    let token_id = axelar_solana_its::linked_token_id(&ctx.solana_wallet, &salt);
    let register_custom_token_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        salt,
        solana_token,
        TokenManagerType::MintBurnFrom,
        spl_token_2022::id(),
        None,
    )?;

    ctx.send_solana_tx(&[register_custom_token_ix])
        .await
        .unwrap();

    // Link the token
    let link_token_ix = axelar_solana_its::instruction::link_token(
        ctx.solana_wallet,
        salt,
        ctx.evm_chain_name.clone(),
        custom_token.address().as_bytes().to_vec(),
        TokenManagerType::MintBurnFrom,
        vec![],
        0,
    )?;

    // Check that an invalid program account leads to a failure
    {
        let mut link_token_ix = link_token_ix.clone();
        link_token_ix.accounts[9].pubkey = Pubkey::new_unique();
        let result = ctx.send_solana_tx(&[link_token_ix]).await;
        assert!(result.is_err());

        assert_msg_present_in_logs(
            result.unwrap_err(),
            "failed: incorrect program id for instruction",
        );
    }

    let tx = ctx.send_solana_tx(&[link_token_ix]).await.unwrap();
    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);
    ctx.relay_to_evm(&call_contract_event.payload).await;

    // Transfer mint authority to ITS
    let authority_transfer_ix =
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            ctx.solana_wallet,
            token_id,
            solana_token,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[authority_transfer_ix]).await.unwrap();

    // Transfer mintership on EVM side
    let token_manager = ctx
        .evm_its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .call()
        .await?;

    custom_token
        .transfer_mintership(token_manager)
        .send()
        .await?
        .await?;

    // Create bob's keypair and token account
    let bob = solana_sdk::signer::keypair::Keypair::new();
    let bob_token_account = get_associated_token_address_with_program_id(
        &bob.pubkey(),
        &solana_token,
        &spl_token_2022::id(),
    );

    // Fund bob's account for transaction fees
    ctx.solana_chain
        .fixture
        .fund_account(&bob.pubkey(), 10_000_000_000)
        .await;

    // Create bob's associated token account
    let create_bob_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &bob.pubkey(),
        &solana_token,
        &spl_token_2022::id(),
    );

    // Mint tokens to bob through ITS
    let mint_amount = 1000;
    let mint_to_bob_ix = axelar_solana_its::instruction::interchain_token::mint(
        ctx.solana_wallet,
        token_id,
        solana_token,
        bob.pubkey(),
        ctx.solana_wallet,
        spl_token_2022::id(),
        mint_amount,
    )?;

    ctx.send_solana_tx(&[create_bob_ata_ix, mint_to_bob_ix])
        .await
        .unwrap();

    // Have bob approve solana_wallet for a certain amount
    let approve_amount = 500;
    let approve_ix = spl_token_2022::instruction::approve(
        &spl_token_2022::id(),
        &bob_token_account,
        &ctx.solana_wallet,
        &bob.pubkey(),
        &[],
        approve_amount,
    )?;

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[approve_ix],
            &[
                bob.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    // Make solana_wallet perform an interchain transfer from bob's account using approved amount
    let transfer_amount = 300;
    let interchain_transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        bob.pubkey(),
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        solana_token,
        spl_token_2022::id(),
        0,
    )?;

    let tx = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[interchain_transfer_ix],
            &[
                ctx.solana_chain.fixture.payer.insecure_clone(),
                bob.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Verify the transfer was successful by checking bob's balance
    let bob_account_data = ctx
        .solana_chain
        .fixture
        .try_get_account_no_checks(&bob_token_account)
        .await
        .unwrap()
        .unwrap()
        .data;
    let bob_token_account_info =
        spl_token_2022::state::Account::unpack_from_slice(&bob_account_data)?;
    assert_eq!(bob_token_account_info.amount, mint_amount - transfer_amount);

    // Verify bob still has delegate set to solana_wallet with remaining allowance
    assert_eq!(bob_token_account_info.delegate.unwrap(), ctx.solana_wallet);
    assert_eq!(
        bob_token_account_info.delegated_amount,
        approve_amount - transfer_amount
    );

    // Verify the event was emitted
    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);
    let GMPPayload::SendToHub(hub_message) = GMPPayload::decode(&call_contract_event.payload)?
    else {
        panic!("wrong message");
    };

    let GMPPayload::InterchainTransfer(transfer_payload) =
        GMPPayload::decode(&hub_message.payload)?
    else {
        panic!("wrong message");
    };

    assert_eq!(transfer_payload.token_id, token_id);
    assert_eq!(transfer_payload.amount, transfer_amount.try_into().unwrap());
    assert_eq!(
        transfer_payload.destination_address.as_ref(),
        ctx.evm_signer.wallet.address().as_bytes()
    );

    // Relay to EVM and verify
    ctx.relay_to_evm(&call_contract_event.payload).await;

    // Verify tokens were minted on EVM side
    let evm_balance = custom_token
        .balance_of(ctx.evm_signer.wallet.address())
        .call()
        .await?;
    assert_eq!(evm_balance, transfer_amount.into());

    Ok(())
}

/// Test that an one cannot pass an arbitrary token_manager_ata
/// to process_outbound_transfer for LockUnlock token managers.
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_ata_must_match_pda_derivation(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let initial_balance = 300;
    let (token_id, evm_token, solana_token) =
        custom_token(ctx, TokenManagerType::LockUnlock).await?;

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    {
        let create_ata_ix = create_associated_token_account(
            &ctx.solana_wallet,
            &ctx.solana_wallet,
            &solana_token,
            &spl_token_2022::id(),
        );

        let mint_ix = spl_token_2022::instruction::mint_to(
            &spl_token_2022::id(),
            &solana_token,
            &token_account,
            &ctx.solana_wallet,
            &[],
            initial_balance,
        )?;

        ctx.send_solana_tx(&[create_ata_ix, mint_ix]).await.unwrap();
    }

    // Mint some tokens to the token manager so it can unlock
    {
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
    }

    let mut transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        initial_balance,
        solana_token,
        spl_token_2022::id(),
        0,
    )
    .unwrap();

    // Now inject an arbitrary ATA that does not match the token manager PDA
    transfer_ix.accounts[5].pubkey = {
        let attacker_wallet = Keypair::new();

        // Fund the attacker wallet (for transaction fees)
        ctx.solana_chain
            .fixture
            .fund_account(&attacker_wallet.pubkey(), 1_000_000)
            .await;

        // Create the attacker's ATA
        let create_attacker_ata_ix = create_associated_token_account(
            &ctx.solana_wallet,        // payer
            &attacker_wallet.pubkey(), // owner
            &solana_token,
            &spl_token_2022::id(),
        );
        ctx.send_solana_tx(&[create_attacker_ata_ix]).await.unwrap();

        get_associated_token_address_with_program_id(
            &attacker_wallet.pubkey(),
            &solana_token,
            &spl_token_2022::id(),
        )
    };

    let res = ctx.send_solana_tx(&[transfer_ix]).await;

    assert!(res.is_err());
    assert_msg_present_in_logs(
        res.err().unwrap(),
        "Provided token_manager_ata doesn't match expected derivation",
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_source_address_stays_consistent_through_the_transfer(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let salt = solana_sdk::keccak::hash(b"SourceAddressTestToken").0;
    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Source Test Token".to_owned(),
        "STT".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    ctx.send_solana_tx(&[deploy_local_ix]).await.unwrap();

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_mint, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    // Perform interchain transfer to verify source_address
    let transfer_amount = 50;
    let destination_address = b"0x1234567890123456789012345678901234567890".to_vec();

    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        token_id,
        ctx.evm_chain_name.clone(),
        destination_address.clone(),
        transfer_amount,
        interchain_token_mint,
        spl_token_2022::id(),
        0,
    )?;

    let tx = ctx.send_solana_tx(&[transfer_ix]).await.unwrap();

    // Extract the CallContract event to get the GMP payload first
    let call_contract_event = fetch_first_call_contract_event_from_tx(&tx);

    let gmp_payload = GMPPayload::decode(&call_contract_event.payload)?;

    // Extract the InterchainTransfer event from logs
    let logs = tx.metadata.unwrap().log_messages;
    let transfer_event = logs
        .iter()
        .find_map(|log| InterchainTransfer::try_from_log(log).ok())
        .expect("InterchainTransfer event should be present");

    // Extract the InterchainTransfer from the GMP payload
    let GMPPayload::SendToHub(send_to_hub) = gmp_payload else {
        panic!("Expected SendToHub GMP payload, got: {gmp_payload:?}");
    };

    // The actual InterchainTransfer is in the inner payload
    let inner_gmp_payload = GMPPayload::decode(&send_to_hub.payload)?;
    let GMPPayload::InterchainTransfer(gmp_transfer) = inner_gmp_payload else {
        panic!("Expected InterchainTransfer in inner payload, got: {inner_gmp_payload:?}");
    };

    // Both event and GMP payload should use the same source address
    // and it should be the user's token account (not token manager ATA or mint)
    assert_eq!(
        transfer_event.source_address, ctx.solana_wallet,
        "Event source_address should be the user's token account"
    );

    assert_eq!(
        gmp_transfer.source_address.as_ref(),
        ctx.solana_wallet.to_bytes(),
        "GMP payload source_address should be the user's token account"
    );

    // Verify that the source address is NOT any system account
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &interchain_token_mint,
        &spl_token_2022::id(),
    );

    assert_ne!(
        transfer_event.source_address, token_manager_ata,
        "Source address should NOT be token manager ATA"
    );

    assert_ne!(
        transfer_event.source_address, interchain_token_mint,
        "Source address should NOT be the mint account"
    );

    // Additional verification: check other event fields
    assert_eq!(transfer_event.token_id, token_id);
    assert_eq!(transfer_event.destination_chain, ctx.evm_chain_name);
    assert_eq!(transfer_event.destination_address, destination_address);
    assert_eq!(transfer_event.amount, transfer_amount);

    Ok(())
}
