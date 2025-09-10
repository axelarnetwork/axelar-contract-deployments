use borsh::BorshDeserialize;
use event_utils::Event;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program_test::tokio;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::solana_program::program_pack::Pack;
use test_context::test_context;

use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_its::state::token_manager::{self, TokenManager};
use axelar_solana_its::Roles;
use role_management::state::UserRoles;

use crate::{BorshPdaAccount, ItsTestContext};

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_handover_mint_authority_exploit_prevention(ctx: &mut ItsTestContext) {
    // Bob is a malicious actor
    let bob = Keypair::new();

    // Fund Bob's account
    ctx.send_solana_tx(&[system_instruction::transfer(
        &ctx.solana_chain.fixture.payer.pubkey(),
        &bob.pubkey(),
        u32::MAX.into(),
    )])
    .await
    .unwrap();

    // Step 1: Bob creates a new token (TokenB) on the Solana blockchain
    let token_b_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(bob.pubkey(), spl_token_2022::id(), 9)
        .await;

    let (metadata_pda, _) = mpl_token_metadata::accounts::Metadata::find_pda(&token_b_mint);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(token_b_mint, false)
        .authority(bob.pubkey())
        .update_authority(bob.pubkey(), true)
        .payer(bob.pubkey())
        .is_mutable(false)
        .name("Token B".to_string())
        .symbol("TOKB".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[metadata_ix],
            &[
                &bob.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Step 2: Bob calls RegisterCustomToken instruction to register TokenB
    let salt = solana_sdk::keccak::hash(b"BobsTokenB").0;
    let register_ix = axelar_solana_its::instruction::register_custom_token(
        bob.pubkey(),
        salt,
        token_b_mint,
        token_manager::Type::LockUnlock,
        spl_token_2022::id(),
        Some(bob.pubkey()),
    )
    .unwrap();

    let tx = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[register_ix],
            &[
                &bob.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Get Bob's token_id from the transaction events
    let bob_token_id_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenIdClaimed::try_from_log(log).ok())
        .unwrap();

    let bob_token_id = bob_token_id_event.token_id;

    // Verify Bob's TokenManager was created
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (bob_token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &bob_token_id);

    let bob_token_manager_data = ctx
        .solana_chain
        .fixture
        .get_account(&bob_token_manager_pda, &axelar_solana_its::id())
        .await;

    let mut bob_token_manager_account = bob_token_manager_data.clone();
    let bob_token_manager = bob_token_manager_account
        .deserialize::<TokenManager>(&bob_token_manager_pda)
        .unwrap();

    assert_eq!(bob_token_manager.token_address, token_b_mint);

    // Step 3: Get the token_id of the target token (TokenTarget - ctx.deployed_interchain_token)
    let target_token_id = ctx.deployed_interchain_token;
    let (target_token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &target_token_id);

    // Get the target token mint address
    let target_token_manager_data = ctx
        .solana_chain
        .fixture
        .get_account(&target_token_manager_pda, &axelar_solana_its::id())
        .await;

    let mut target_token_manager_account = target_token_manager_data.clone();
    let target_token_manager = target_token_manager_account
        .deserialize::<TokenManager>(&target_token_manager_pda)
        .unwrap();

    let target_token_mint = target_token_manager.token_address;

    // Step 4 & 5: Bob attempts the exploit by calling TokenManagerHandOverMintAuthority
    // with the target token's token_id but providing his own TokenB mint address

    let handover_ix = axelar_solana_its::instruction::token_manager::handover_mint_authority(
        bob.pubkey(),
        target_token_id, // Target token's ID
        token_b_mint,    // Bob's token mint (TokenB)
        spl_token_2022::id(),
    )
    .unwrap();

    // This should fail due to the fix
    let tx_result = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[handover_ix],
            &[
                &bob.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    // Verify the transaction failed with the expected error
    let tx_metadata = match tx_result {
        Ok(_meta) => {
            panic!("Expected transaction to fail but it succeeded");
        }
        Err(meta) => meta,
    };

    assert!(
        tx_metadata
            .find_log("TokenManager PDA does not match the provided Mint account")
            .is_some(),
        "Expected error about TokenManager PDA not matching Mint account"
    );

    // Verify Bob does NOT have minter role on the target token
    let (bob_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &target_token_manager_pda,
        &bob.pubkey(),
    );

    // Account should not exist since Bob never got any roles on the target token
    let bob_roles_result = ctx
        .solana_chain
        .fixture
        .try_get_account(&bob_roles_pda, &axelar_solana_its::id())
        .await
        .unwrap();

    assert!(
        bob_roles_result.is_none(),
        "Bob should not have any roles on the target token"
    );

    // Also verify that Bob still has mint authority on his own token (TokenB)
    let token_b_mint_data = ctx
        .solana_chain
        .fixture
        .get_account(&token_b_mint, &spl_token_2022::id())
        .await
        .data;

    let token_b_mint_state = spl_token_2022::state::Mint::unpack(&token_b_mint_data).unwrap();
    assert_eq!(
        token_b_mint_state.mint_authority.unwrap(),
        bob.pubkey(),
        "Bob should still be the mint authority of TokenB"
    );

    // Verify the target token's mint authority is unchanged (should still be the token manager)
    let target_mint_data = ctx
        .solana_chain
        .fixture
        .get_account(&target_token_mint, &spl_token_2022::id())
        .await
        .data;

    let target_mint_state = spl_token_2022::state::Mint::unpack(&target_mint_data).unwrap();
    assert_eq!(
        target_mint_state.mint_authority.unwrap(),
        target_token_manager_pda,
        "Target token mint authority should still be the token manager"
    );
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_successful_handover_mint_authority(ctx: &mut ItsTestContext) {
    let alice = Keypair::new();

    ctx.send_solana_tx(&[system_instruction::transfer(
        &ctx.solana_chain.fixture.payer.pubkey(),
        &alice.pubkey(),
        u32::MAX.into(),
    )])
    .await
    .unwrap();

    let alice_token_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(alice.pubkey(), spl_token_2022::id(), 9)
        .await;

    let (metadata_pda, _) = mpl_token_metadata::accounts::Metadata::find_pda(&alice_token_mint);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(alice_token_mint, false)
        .authority(alice.pubkey())
        .update_authority(alice.pubkey(), true)
        .payer(alice.pubkey())
        .is_mutable(false)
        .name("Alice Token".to_string())
        .symbol("ALICE".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[metadata_ix],
            &[
                &alice.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let salt = solana_sdk::keccak::hash(b"AliceToken").0;
    let register_ix = axelar_solana_its::instruction::register_custom_token(
        alice.pubkey(),
        salt,
        alice_token_mint,
        token_manager::Type::LockUnlock,
        spl_token_2022::id(),
        Some(alice.pubkey()),
    )
    .unwrap();

    let tx = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[register_ix],
            &[
                &alice.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let alice_token_id_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenIdClaimed::try_from_log(log).ok())
        .unwrap();

    let alice_token_id = alice_token_id_event.token_id;
    let handover_ix = axelar_solana_its::instruction::token_manager::handover_mint_authority(
        alice.pubkey(),
        alice_token_id,   // Alice's own token ID
        alice_token_mint, // Alice's own token mint
        spl_token_2022::id(),
    )
    .unwrap();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[handover_ix],
            &[
                &alice.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (alice_token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &alice_token_id);

    let (alice_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &alice_token_manager_pda,
        &alice.pubkey(),
    );

    let alice_roles_data = ctx
        .solana_chain
        .fixture
        .get_account(&alice_roles_pda, &axelar_solana_its::id())
        .await;

    let alice_roles = UserRoles::<Roles>::try_from_slice(&alice_roles_data.data).unwrap();
    assert!(
        alice_roles.contains(Roles::MINTER),
        "Alice should have minter role after handover"
    );

    let alice_mint_data = ctx
        .solana_chain
        .fixture
        .get_account(&alice_token_mint, &spl_token_2022::id())
        .await
        .data;

    let alice_mint_state = spl_token_2022::state::Mint::unpack(&alice_mint_data).unwrap();
    assert_eq!(
        alice_mint_state.mint_authority.unwrap(),
        alice_token_manager_pda,
        "Token manager should be the mint authority after handover"
    );

    let alice_ata = get_associated_token_address_with_program_id(
        &alice.pubkey(),
        &alice_token_mint,
        &spl_token_2022::id(),
    );

    let mint_amount = 1000u64;
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        ctx.solana_wallet,
        alice_token_id,
        alice_token_mint,
        alice.pubkey(),
        alice.pubkey(),
        spl_token_2022::id(),
        mint_amount,
    )
    .unwrap();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[mint_ix],
            &[
                &alice.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let alice_ata_account = ctx
        .solana_chain
        .fixture
        .try_get_account_no_checks(&alice_ata)
        .await
        .unwrap()
        .unwrap();

    let alice_ata_state =
        spl_token_2022::state::Account::unpack_from_slice(&alice_ata_account.data).unwrap();
    assert_eq!(
        alice_ata_state.amount, mint_amount,
        "Alice should have the minted tokens"
    );
}
