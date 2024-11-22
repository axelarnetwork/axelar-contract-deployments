use axelar_solana_its::instructions::DeployInterchainTokenInputs;
use role_management::state::{Roles, UserRoles};
use solana_program_test::tokio;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::{keccak, system_instruction};

use crate::{axelar_solana_setup, program_test, ItsProgramWrapper};

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
    let bob_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&bob_roles_pda, &axelar_solana_its::id())
        .await;

    assert!(bob_roles.contains(Roles::OPERATOR));

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &its_root_pda,
        &solana_chain.fixture.payer.pubkey(),
    );
    let alice_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&alice_roles_pda, &axelar_solana_its::id())
        .await;

    assert!(!alice_roles.contains(Roles::OPERATOR));
}

#[tokio::test]
#[should_panic(expected = "assertion failed: tx.result.is_ok()")]
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

    solana_chain.fixture.send_tx(&[transfer_role_ix]).await;
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
    let alice_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&alice_roles_pda, &axelar_solana_its::id())
        .await;

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

    let new_alice_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&alice_roles_pda, &axelar_solana_its::id())
        .await;

    // Alice should not have the roles anymore
    assert!(!new_alice_roles.contains(roles_to_transfer));

    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &its_root_pda,
        &bob.pubkey(),
    );
    let bob_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&bob_roles_pda, &axelar_solana_its::id())
        .await;

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
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(&interchain_token_pda);
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

    let bob_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&bob_roles_pda, &axelar_solana_its::id())
        .await;

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

    let bob_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&bob_roles_pda, &axelar_solana_its::id())
        .await;

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &alice.pubkey(),
    );
    let alice_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&alice_roles_pda, &axelar_solana_its::id())
        .await;

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
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(&interchain_token_pda);
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

    let bob_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&bob_roles_pda, &axelar_solana_its::id())
        .await;

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

    let bob_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&bob_roles_pda, &axelar_solana_its::id())
        .await;

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

    let bob_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&bob_roles_pda, &axelar_solana_its::id())
        .await;

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &alice.pubkey(),
    );

    let alice_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&alice_roles_pda, &axelar_solana_its::id())
        .await;

    assert!(!bob_roles.contains(Roles::OPERATOR));
    assert!(alice_roles.contains(Roles::OPERATOR));
}
