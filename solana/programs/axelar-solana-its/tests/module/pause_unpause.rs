#![allow(clippy::unwrap_used)]
#![allow(clippy::should_panic_without_expect)]
use alloy_sol_types::SolValue;
use axelar_solana_its::{instructions::DeployTokenManagerInputs, state::token_manager};
use evm_contracts_test_suite::ethers::abi::Bytes;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use crate::{
    prepare_receive_from_hub, program_test, random_hub_message_with_destination_and_payload,
};

#[tokio::test]
#[should_panic]
async fn test_its_gmp_payload_fail_when_paused() {
    use axelar_solana_its::instructions::ItsGmpInstructionInputs;

    let mut solana_chain = program_test().await;
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instructions::set_pause_status(
                solana_chain.upgrade_authority.pubkey(),
                true,
            )
            .unwrap()],
            &[
                solana_chain.upgrade_authority.insecure_clone(),
                solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let token_program_id = spl_token_2022::id();
    let token_id =
        Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id()).unwrap();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, token_id.as_ref());
    let mint_authority = axelar_solana_its::find_token_manager_pda(&interchain_token_pda).0;
    let mint = solana_chain
        .fixture
        .init_new_mint(mint_authority, token_program_id, 18)
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.to_bytes().into(),
        token_manager_type: alloy_primitives::Uint::<256, 4>::from(4_u128),
        params: axelar_solana_its::state::token_manager::encode_params(
            None,
            Some(solana_chain.fixture.payer.pubkey()),
            mint,
        )
        .into(),
    });

    let its_gmp_payload = prepare_receive_from_hub(&inner_payload, "ethereum".to_owned());
    let abi_payload = its_gmp_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );
    // Action: "Relayer" calls Gateway to approve messages
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            vec![message.clone()],
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .gateway_approved_message_pda(gateway_approved_command_pdas[0])
        .gateway_root_pda(solana_chain.gateway_root_pda)
        .gmp_metadata(message.into())
        .payload(its_gmp_payload)
        .token_program(token_program_id)
        .build();

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap()])
        .await;
}

#[tokio::test]
#[should_panic]
async fn test_outbound_deployment_fails_when_paused() {
    let mut solana_chain = program_test().await;
    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instructions::set_pause_status(
                solana_chain.upgrade_authority.pubkey(),
                true,
            )
            .unwrap()],
            &[
                solana_chain.upgrade_authority.insecure_clone(),
                solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let token_address = alloy_primitives::Address::new([0x00; 20]);
    let params = (Bytes::new(), token_address).abi_encode_params();

    let destination_chain = "ethereum".to_string();
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let deploy = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .destination_chain(destination_chain)
        .token_manager_type(token_manager::Type::LockUnlock)
        .gas_value(0)
        .params(params)
        .build();

    let ix = axelar_solana_its::instructions::deploy_token_manager(deploy.clone()).unwrap();
    solana_chain.fixture.send_tx(&[ix]).await;
}

#[tokio::test]
#[should_panic]
async fn test_fail_to_pause_not_being_owner() {
    let mut solana_chain = program_test().await;
    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instructions::set_pause_status(
                solana_chain.fixture.payer.pubkey(),
                true,
            )
            .unwrap()],
            &[solana_chain.fixture.payer.insecure_clone()],
        )
        .await;
}
