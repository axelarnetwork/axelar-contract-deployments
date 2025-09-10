use borsh::BorshDeserialize;
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use test_context::test_context;

use axelar_solana_gateway_test_fixtures::base::FindLog;
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

    let tx_metadata = ctx.relay_to_solana(&payload, None, token_program_id).await;

    assert!(tx_metadata
        .find_log("The Interchain Token Service is currently paused.")
        .is_some());
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

    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        ctx.solana_wallet,
        ctx.deployed_interchain_token,
        token_address,
        ctx.solana_wallet,
        ctx.solana_wallet,
        spl_token_2022::id(),
        900,
    )
    .unwrap();
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
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
        .send_solana_tx(&[mint_ix, transfer_ix])
        .await
        .unwrap_err();

    assert!(tx_metadata
        .find_log("The Interchain Token Service is currently paused.")
        .is_some());
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

    assert!(tx_metadata
        .find_log("Given authority is not the program upgrade authority")
        .is_some());
}
