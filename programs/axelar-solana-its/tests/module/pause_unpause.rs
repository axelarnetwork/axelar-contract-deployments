use borsh::BorshDeserialize;
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use test_context::test_context;

use axelar_solana_gateway_test_fixtures::assert_msg_present_in_logs;
use axelar_solana_its::state::token_manager::TokenManager;
use evm_contracts_test_suite::ethers::signers::Signer as _;
use interchain_token_transfer_gmp::{GMPPayload, LinkToken, SendToHub};

use crate::ItsTestContext;

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_its_gmp_payload_fail_when_paused(ctx: &mut ItsTestContext) {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instruction::set_pause_status(
                ctx.solana_chain.upgrade_authority.pubkey(),
                true,
            )
            .unwrap()],
            &[
                ctx.solana_chain.upgrade_authority.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let token_program_id = spl_token_2022::id();
    let token_id = Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id())
        .unwrap()
        .to_bytes();
    let (mint_authority, _) = axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(mint_authority, token_program_id, 18)
        .await;

    let payload = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID.try_into().unwrap(),
        payload: GMPPayload::LinkToken(LinkToken {
            selector: LinkToken::MESSAGE_TYPE_ID.try_into().unwrap(),
            token_id: token_id.into(),
            token_manager_type: alloy_primitives::Uint::<256, 4>::from(4_u128),
            link_params: vec![].into(),
            source_token_address: [0; 20].into(),
            destination_token_address: mint.to_bytes().into(),
        })
        .encode()
        .into(),
        destination_chain: ctx.solana_chain_name.clone(),
    })
    .encode();

    let (_inner_ixs, tx_metadata) = ctx.relay_to_solana(&payload, None, token_program_id).await;
    assert_msg_present_in_logs(
        tx_metadata,
        "The Interchain Token Service is currently paused.",
    );
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_outbound_message_fails_when_paused(ctx: &mut ItsTestContext) {
    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instruction::set_pause_status(
                ctx.solana_chain.upgrade_authority.pubkey(),
                true,
            )
            .unwrap()],
            &[
                ctx.solana_chain.upgrade_authority.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let (its_root_config_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(
        &its_root_config_pda,
        &ctx.deployed_interchain_token,
    );
    let data = ctx
        .solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;

    let token_manager = TokenManager::try_from_slice(&data).unwrap();
    let token_address = token_manager.token_address;

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &token_address,
        &spl_token_2022::id(),
    );

    let create_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &token_address,
        &spl_token_2022::id(),
    );

    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        ctx.deployed_interchain_token,
        token_address,
        token_account,
        ctx.solana_wallet,
        spl_token_2022::id(),
        900,
    )
    .unwrap();
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        token_account,
        ctx.deployed_interchain_token,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        500,
        token_address,
        spl_token_2022::id(),
        0,
    )
    .unwrap();

    let tx_metadata = ctx
        .send_solana_tx(&[create_ata_ix, mint_ix, transfer_ix])
        .await
        .unwrap_err();
    assert_msg_present_in_logs(
        tx_metadata,
        "The Interchain Token Service is currently paused.",
    );
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_fail_to_pause_not_being_owner(ctx: &mut ItsTestContext) {
    let tx_metadata = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instruction::set_pause_status(
                ctx.solana_chain.fixture.payer.pubkey(),
                true,
            )
            .unwrap()],
            &[ctx.solana_chain.fixture.payer.insecure_clone()],
        )
        .await
        .unwrap_err();

    assert_msg_present_in_logs(
        tx_metadata,
        "Given authority is not the program upgrade authority",
    );
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_inbound_deploy_interchain_token_fails_when_paused(ctx: &mut ItsTestContext) {
    use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;

    // First pause the ITS
    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instruction::set_pause_status(
                ctx.solana_chain.upgrade_authority.pubkey(),
                true,
            )
            .unwrap()],
            &[
                ctx.solana_chain.upgrade_authority.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    // Deploy an interchain token on EVM
    let salt = [42u8; 32];
    let token_name = "Test Token";
    let token_symbol = "TEST";

    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_interchain_token(
            salt,
            token_name.to_string(),
            token_symbol.to_string(),
            18,       // EVM decimals
            0.into(), // initial supply
            ctx.evm_signer.wallet.address(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    // Deploy remotely to Solana
    ctx.evm_its_contracts
        .interchain_token_factory
        .deploy_remote_interchain_token(salt, ctx.solana_chain_name.clone(), 0.into())
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    // Capture the contract call
    let log: ContractCallFilter = ctx
        .evm_its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await
        .unwrap()
        .into_iter()
        .last()
        .expect("Should have contract call");

    // Relay the deployment message to Solana (should fail due to pause)
    let (_inner_ixs, tx_metadata) = ctx
        .relay_to_solana(log.payload.as_ref(), None, spl_token_2022::id())
        .await;

    // Verify it failed with the paused message
    assert_msg_present_in_logs(
        tx_metadata,
        "The Interchain Token Service is currently paused.",
    );
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_local_deploy_interchain_token_fails_when_paused(ctx: &mut ItsTestContext) {
    // First pause the ITS
    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instruction::set_pause_status(
                ctx.solana_chain.upgrade_authority.pubkey(),
                true,
            )
            .unwrap()],
            &[
                ctx.solana_chain.upgrade_authority.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    // Try to deploy an interchain token locally (should fail when paused)
    let salt = solana_sdk::keccak::hash(b"LocalTokenWhilePaused").0;
    let token_name = "Local Token";
    let token_symbol = "LOCAL";
    let decimals = 9;
    let initial_supply = 1_000_000;
    let minter = Some(ctx.solana_wallet);

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        token_name.to_owned(),
        token_symbol.to_owned(),
        decimals,
        initial_supply,
        minter,
    )
    .unwrap();

    // Local deployment should fail when paused
    let result = ctx.send_solana_tx(&[deploy_local_ix]).await;

    assert!(result.is_err(), "Local deployment should fail when paused");

    // Verify it failed with the paused message
    let metadata = result.unwrap_err();
    assert_msg_present_in_logs(
        metadata,
        "The Interchain Token Service is currently paused.",
    );
}
