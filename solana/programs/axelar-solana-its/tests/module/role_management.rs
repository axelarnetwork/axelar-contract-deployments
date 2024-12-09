#![allow(clippy::should_panic_without_expect)]

use axelar_solana_gateway::{get_incoming_message_pda, state::incoming_message::command_id};
use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_its::instructions::DeployInterchainTokenInputs;
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use axelar_solana_its::Roles;
use borsh::BorshDeserialize;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use role_management::state::UserRoles;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::{keccak, system_instruction};

use crate::{prepare_receive_from_hub, random_hub_message_with_destination_and_payload};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};

use crate::{axelar_solana_setup, program_test, ItsProgramWrapper, TokenUtils};

#[tokio::test]
async fn test_successful_operator_transfer() {
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

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let bob = Keypair::new();

    let transfer_role_ix = axelar_solana_its::instructions::transfer_operatorship(
        solana_chain.fixture.payer.pubkey(),
        bob.pubkey(),
    )
    .unwrap();

    solana_chain.fixture.send_tx(&[transfer_role_ix]).await;

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &its_root_pda,
        &bob.pubkey(),
    );
    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;

    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::OPERATOR));

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &its_root_pda,
        &solana_chain.fixture.payer.pubkey(),
    );
    let data = solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let alice_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(!alice_roles.contains(Roles::OPERATOR));
}

#[tokio::test]
async fn test_fail_transfer_when_not_holder() {
    let mut solana_chain = program_test().await;

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            Keypair::new().pubkey(),
        )
        .unwrap()])
        .await;

    let bob = Keypair::new();

    // We don't have the role, so this should fail
    let transfer_role_ix = axelar_solana_its::instructions::transfer_operatorship(
        solana_chain.fixture.payer.pubkey(),
        bob.pubkey(),
    )
    .unwrap();

    let tx_metadata = solana_chain
        .fixture
        .send_tx(&[transfer_role_ix])
        .await
        .unwrap_err();
    assert!(tx_metadata
        .find_log("User roles account not found")
        .is_some());
}

#[tokio::test]
async fn test_successful_proposal_acceptance() {
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

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let bob = Keypair::new();

    let roles_to_transfer = Roles::OPERATOR;

    let proposal_ix = axelar_solana_its::instructions::propose_operatorship(
        solana_chain.fixture.payer.pubkey(),
        bob.pubkey(),
    )
    .unwrap();

    solana_chain.fixture.send_tx(&[proposal_ix]).await;

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &its_root_pda,
        &solana_chain.fixture.payer.pubkey(),
    );
    let data = solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let alice_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    // Alice should still have the roles
    assert!(alice_roles.contains(roles_to_transfer));

    let accept_ix = axelar_solana_its::instructions::accept_operatorship(
        bob.pubkey(),
        solana_chain.fixture.payer.pubkey(),
    )
    .unwrap();
    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[
                // First transfer funds to bob so he can pay for the user role account
                system_instruction::transfer(
                    &solana_chain.fixture.payer.pubkey(),
                    &bob.pubkey(),
                    u32::MAX.into(),
                ),
                accept_ix,
            ],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let data = solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let new_alice_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    // Alice should not have the roles anymore
    assert!(!new_alice_roles.contains(roles_to_transfer));

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &its_root_pda,
        &bob.pubkey(),
    );
    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    // Bob should have the roles now
    assert!(bob_roles.contains(roles_to_transfer));
}

#[tokio::test]
async fn test_successful_add_and_remove_flow_limiter() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;

    let bob = Keypair::new();
    let salt = keccak::hash(b"our cool token").0;
    let token_name = "MyToken";
    let token_symbol = "MTK";
    let deploy_instruction = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .decimals(18)
        .salt(salt)
        .minter(bob.pubkey().as_ref().to_vec())
        .gas_value(0)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            // First transfer funds to bob so he can pay for the user role account
            system_instruction::transfer(
                &solana_chain.fixture.payer.pubkey(),
                &bob.pubkey(),
                u32::MAX.into(),
            ),
            axelar_solana_its::instructions::deploy_interchain_token(deploy_instruction).unwrap(),
        ])
        .await;

    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let add_flow_limiter_ix = axelar_solana_its::instructions::token_manager::add_flow_limiter(
        bob.pubkey(),
        token_id,
        solana_chain.fixture.payer.pubkey(),
    )
    .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[add_flow_limiter_ix],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let remove_flow_limiter_ix =
        axelar_solana_its::instructions::token_manager::remove_flow_limiter(
            bob.pubkey(),
            token_id,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[remove_flow_limiter_ix],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;
}

#[tokio::test]
async fn test_successful_token_manager_operator_transfer() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;

    let bob = Keypair::new();
    let alice = Keypair::new();
    let salt = keccak::hash(b"our cool token").0;
    let token_name = "MyToken";
    let token_symbol = "MTK";
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let deploy_instruction = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .decimals(18)
        .salt(salt)
        .minter(bob.pubkey().as_ref().to_vec())
        .gas_value(0)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            // First transfer funds to bob so he can pay for the user role account
            system_instruction::transfer(
                &solana_chain.fixture.payer.pubkey(),
                &bob.pubkey(),
                u32::MAX.into(),
            ),
            axelar_solana_its::instructions::deploy_interchain_token(deploy_instruction).unwrap(),
        ])
        .await;

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &bob.pubkey(),
    );

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::OPERATOR));

    let transfer_operatorship_ix =
        axelar_solana_its::instructions::token_manager::transfer_operatorship(
            bob.pubkey(),
            token_id,
            alice.pubkey(),
        )
        .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[transfer_operatorship_ix],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &alice.pubkey(),
    );
    let data = solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let alice_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(!bob_roles.contains(Roles::OPERATOR));
    assert!(alice_roles.contains(Roles::OPERATOR));
}

#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn test_successful_token_manager_operator_proposal_acceptance() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;

    let bob = Keypair::new();
    let alice = Keypair::new();
    let salt = keccak::hash(b"our cool token").0;
    let token_name = "MyToken";
    let token_symbol = "MTK";
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let deploy_instruction = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .decimals(18)
        .salt(salt)
        .minter(bob.pubkey().as_ref().to_vec())
        .gas_value(0)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            system_instruction::transfer(
                &solana_chain.fixture.payer.pubkey(),
                &bob.pubkey(),
                u32::MAX.into(),
            ),
            axelar_solana_its::instructions::deploy_interchain_token(deploy_instruction).unwrap(),
        ])
        .await;

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &bob.pubkey(),
    );

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::OPERATOR));

    let propose_operatorship_ix =
        axelar_solana_its::instructions::token_manager::propose_operatorship(
            bob.pubkey(),
            token_id,
            alice.pubkey(),
        )
        .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[propose_operatorship_ix],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::OPERATOR));

    let accept_operatorship_ix =
        axelar_solana_its::instructions::token_manager::accept_operatorship(
            alice.pubkey(),
            token_id,
            bob.pubkey(),
        )
        .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[
                system_instruction::transfer(
                    &solana_chain.fixture.payer.pubkey(),
                    &alice.pubkey(),
                    u32::MAX.into(),
                ),
                accept_operatorship_ix,
            ],
            &[
                &alice.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &alice.pubkey(),
    );

    let data = solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let alice_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(!bob_roles.contains(Roles::OPERATOR));
    assert!(alice_roles.contains(Roles::OPERATOR));
}

#[tokio::test]
async fn test_successful_token_manager_minter_transfer() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;

    let bob = Keypair::new();
    let alice = Keypair::new();
    let salt = keccak::hash(b"our cool token").0;
    let token_name = "MyToken";
    let token_symbol = "MTK";
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let deploy_instruction = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .decimals(18)
        .salt(salt)
        .minter(bob.pubkey().as_ref().to_vec())
        .gas_value(0)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            // First transfer funds to bob so he can pay for the user role account
            system_instruction::transfer(
                &solana_chain.fixture.payer.pubkey(),
                &bob.pubkey(),
                u32::MAX.into(),
            ),
            axelar_solana_its::instructions::deploy_interchain_token(deploy_instruction).unwrap(),
        ])
        .await;

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &bob.pubkey(),
    );

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::MINTER));

    let transfer_mintership_ix =
        axelar_solana_its::instructions::interchain_token::transfer_mintership(
            bob.pubkey(),
            token_id,
            alice.pubkey(),
        )
        .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[transfer_mintership_ix],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &alice.pubkey(),
    );
    let data = solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let alice_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(!bob_roles.contains(Roles::MINTER));
    assert!(alice_roles.contains(Roles::MINTER));
}

#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn test_successful_token_manager_minter_proposal_acceptance() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;

    let bob = Keypair::new();
    let alice = Keypair::new();
    let salt = keccak::hash(b"our cool token").0;
    let token_name = "MyToken";
    let token_symbol = "MTK";
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let deploy_instruction = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .decimals(18)
        .salt(salt)
        .minter(bob.pubkey().as_ref().to_vec())
        .gas_value(0)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            system_instruction::transfer(
                &solana_chain.fixture.payer.pubkey(),
                &bob.pubkey(),
                u32::MAX.into(),
            ),
            axelar_solana_its::instructions::deploy_interchain_token(deploy_instruction).unwrap(),
        ])
        .await;

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &bob.pubkey(),
    );

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::MINTER));

    let propose_mintership_ix =
        axelar_solana_its::instructions::interchain_token::propose_mintership(
            bob.pubkey(),
            token_id,
            alice.pubkey(),
        )
        .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[propose_mintership_ix],
            &[
                &bob.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::MINTER));

    let accept_mintership_ix =
        axelar_solana_its::instructions::interchain_token::accept_mintership(
            alice.pubkey(),
            token_id,
            bob.pubkey(),
        )
        .unwrap();

    solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[
                system_instruction::transfer(
                    &solana_chain.fixture.payer.pubkey(),
                    &alice.pubkey(),
                    u32::MAX.into(),
                ),
                accept_mintership_ix,
            ],
            &[
                &alice.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &alice.pubkey(),
    );

    let data = solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let alice_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(!bob_roles.contains(Roles::MINTER));
    assert!(alice_roles.contains(Roles::MINTER));
}

#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn test_fail_token_manager_minter_proposal_acceptance() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;

    let bob = Keypair::new();
    let alice = Keypair::new();
    let salt = keccak::hash(b"our cool token").0;
    let token_name = "MyToken";
    let token_symbol = "MTK";
    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);

    solana_chain
        .fixture
        .send_tx(&[
            system_instruction::transfer(
                &solana_chain.fixture.payer.pubkey(),
                &bob.pubkey(),
                u32::MAX.into(),
            ),
            system_instruction::transfer(
                &solana_chain.fixture.payer.pubkey(),
                &alice.pubkey(),
                u32::MAX.into(),
            ),
        ])
        .await
        .unwrap();

    let deploy_instruction = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .decimals(18)
        .salt(salt)
        .minter(bob.pubkey().as_ref().to_vec())
        .gas_value(0)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            axelar_solana_its::instructions::deploy_interchain_token(deploy_instruction).unwrap(),
        ])
        .await
        .unwrap();

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &bob.pubkey(),
    );

    let data = solana_chain
        .fixture
        .get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .data;
    let bob_roles = UserRoles::<Roles>::try_from_slice(&data).unwrap();

    assert!(bob_roles.contains(Roles::MINTER));
    // Trying to accept role that wasn't proposed should fail
    let accept_mintership_ix =
        axelar_solana_its::instructions::interchain_token::accept_mintership(
            alice.pubkey(),
            token_id,
            bob.pubkey(),
        )
        .unwrap();

    let tx_metadata = solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[accept_mintership_ix],
            &[
                &alice.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap_err();

    assert!(tx_metadata
        .find_log("Error: failed to deserialize account as role_management::state::RoleProposal<axelar_solana_its::Roles>")
        .is_some());
}

#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_fail_mint_without_minter_role(#[case] token_program_id: Pubkey) {
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

    let token_id =
        axelar_solana_its::interchain_token_id(&solana_chain.fixture.payer.pubkey(), b"salt");
    let (mint_authority, _) = axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let mint = solana_chain
        .fixture
        .init_new_mint(mint_authority, token_program_id, 18)
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.into(),
        token_manager_type: axelar_solana_its::state::token_manager::Type::MintBurn.into(),
        params: axelar_solana_its::state::token_manager::encode_params(None, None, mint).into(),
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

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap()])
        .await;

    let ata = get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;

    let mint_ix = axelar_solana_its::instructions::interchain_token::mint(
        token_id,
        mint,
        ata,
        solana_chain.fixture.payer.pubkey(),
        token_program_id,
        8000_u64,
    )
    .unwrap();

    let tx_metadata = solana_chain.fixture.send_tx(&[mint_ix]).await.unwrap_err();

    assert!(tx_metadata
        .find_log("User roles account not found")
        .is_some());
}

#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_successful_mint_with_minter_role(#[case] token_program_id: Pubkey) {
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
    let token_id =
        axelar_solana_its::interchain_token_id(&solana_chain.fixture.payer.pubkey(), b"salt");
    let mint_authority = solana_chain.fixture.payer.pubkey();
    let mint = solana_chain
        .fixture
        .init_new_mint(mint_authority, token_program_id, 18)
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.into(),
        token_manager_type: axelar_solana_its::state::token_manager::Type::MintBurn.into(),
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

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap()])
        .await;

    let ata = get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &token_program_id,
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;

    let mint_ix = axelar_solana_its::instructions::interchain_token::mint(
        token_id,
        mint,
        ata,
        solana_chain.fixture.payer.pubkey(),
        token_program_id,
        8000_u64,
    )
    .unwrap();

    solana_chain.fixture.send_tx(&[mint_ix]).await;
}
