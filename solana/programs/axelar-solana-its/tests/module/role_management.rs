use axelar_solana_its::instructions::InterchainTokenServiceInstruction;
use role_management::state::{Roles, UserRoles};
use solana_program_test::tokio;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;

use crate::program_test;

#[tokio::test]
async fn test_successful_operator_transfer() {
    let mut solana_chain = program_test().await;

    dbg!("Initialize");
    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;
    dbg!("Initialized");

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let bob = Keypair::new();
    let roles_to_transfer = Roles::OPERATOR;

    let transfer_role_ix =
        role_management::instructions::transfer_roles::<InterchainTokenServiceInstruction>(
            axelar_solana_its::id(),
            solana_chain.fixture.payer.pubkey(),
            bob.pubkey(),
            roles_to_transfer,
            its_root_pda,
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

    assert!(bob_roles.contains(roles_to_transfer));

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &its_root_pda,
        &solana_chain.fixture.payer.pubkey(),
    );
    let alice_roles = solana_chain
        .fixture
        .get_rkyv_account::<UserRoles>(&alice_roles_pda, &axelar_solana_its::id())
        .await;

    assert!(!alice_roles.contains(roles_to_transfer));
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
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let bob = Keypair::new();

    // We don't have minter role, so this should fail
    let roles_to_transfer = Roles::MINTER | Roles::OPERATOR;

    let transfer_role_ix =
        role_management::instructions::transfer_roles::<InterchainTokenServiceInstruction>(
            axelar_solana_its::id(),
            solana_chain.fixture.payer.pubkey(),
            bob.pubkey(),
            roles_to_transfer,
            its_root_pda,
        )
        .unwrap();

    solana_chain.fixture.send_tx(&[transfer_role_ix]).await;
}

#[tokio::test]
async fn test_successful_add_role() {
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

    let transfer_role_ix =
        role_management::instructions::add_roles::<InterchainTokenServiceInstruction>(
            axelar_solana_its::id(),
            solana_chain.fixture.payer.pubkey(),
            bob.pubkey(),
            roles_to_transfer,
            its_root_pda,
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

    assert!(bob_roles.contains(roles_to_transfer));

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
}

#[tokio::test]
#[should_panic(expected = "assertion failed: tx.result.is_ok()")]
async fn test_fail_add_when_not_holder() {
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
    // We don't have minter role, so this should fail
    let roles_to_transfer = Roles::MINTER | Roles::OPERATOR;

    let transfer_role_ix =
        role_management::instructions::add_roles::<InterchainTokenServiceInstruction>(
            axelar_solana_its::id(),
            solana_chain.fixture.payer.pubkey(),
            bob.pubkey(),
            roles_to_transfer,
            its_root_pda,
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

    let proposal_ix =
        role_management::instructions::propose_roles::<InterchainTokenServiceInstruction>(
            axelar_solana_its::id(),
            solana_chain.fixture.payer.pubkey(),
            bob.pubkey(),
            roles_to_transfer,
            its_root_pda,
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

    let accept_ix =
        role_management::instructions::accept_roles::<InterchainTokenServiceInstruction>(
            axelar_solana_its::id(),
            solana_chain.fixture.payer.pubkey(),
            bob.pubkey(),
            roles_to_transfer,
            its_root_pda,
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
