#![allow(clippy::unwrap_used)]
#![allow(clippy::should_panic_without_expect)]
use alloy_sol_types::SolValue;
use axelar_solana_gateway::{get_incoming_message_pda, state::incoming_message::command_id};
use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_its::instructions::{DeployTokenManagerInputs, ItsGmpInstructionInputs};
use axelar_solana_its::state::token_manager;
use evm_contracts_test_suite::ethers::abi::Bytes;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use crate::{
    prepare_receive_from_hub, program_test, random_hub_message_with_destination_and_payload,
    TokenUtils,
};

#[tokio::test]
async fn test_its_gmp_payload_fail_when_paused() {
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
    let token_id = Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id())
        .unwrap()
        .to_bytes();
    let (mint_authority, _) = axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let mint = solana_chain
        .fixture
        .init_new_mint(mint_authority, token_program_id, 18)
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.into(),
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
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_payload)
        .token_program(token_program_id)
        .build();

    let tx_metadata = solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap()])
        .await
        .unwrap_err();

    assert!(tx_metadata
        .find_log("The Interchain Token Service is currently paused.")
        .is_some());
}

#[tokio::test]
async fn test_outbound_deployment_fails_when_paused() {
    let mut solana_chain = program_test().await;
    let gas_utils = solana_chain.fixture.deploy_gas_service().await;
    solana_chain
        .fixture
        .init_gas_config(&gas_utils)
        .await
        .unwrap();
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
        .gas_service(axelar_solana_gas_service::id())
        .gas_config_pda(gas_utils.config_pda)
        .gas_value(0)
        .params(params)
        .build();

    let ix = axelar_solana_its::instructions::deploy_token_manager(deploy.clone()).unwrap();
    let tx_metadata = solana_chain.fixture.send_tx(&[ix]).await.unwrap_err();

    assert!(tx_metadata
        .find_log("The Interchain Token Service is currently paused.")
        .is_some());
}

#[tokio::test]
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

    let tx_metadata = solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[axelar_solana_its::instructions::set_pause_status(
                solana_chain.fixture.payer.pubkey(),
                true,
            )
            .unwrap()],
            &[solana_chain.fixture.payer.insecure_clone()],
        )
        .await
        .unwrap_err();

    assert!(tx_metadata
        .find_log("Given authority is not the program upgrade authority")
        .is_some());
}
