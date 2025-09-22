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

use axelar_solana_gateway_test_fixtures::assert_msg_present_in_logs;
use axelar_solana_its::state::token_manager::{self, TokenManager};
use axelar_solana_its::Roles;
use role_management::state::UserRoles;

use crate::{BorshPdaAccount, ItsTestContext};

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_handover_mint_authority_exploit_prevention(ctx: &mut ItsTestContext) {
    // First, we need to create a legitimate target token with MintBurn type
    // that Bob will try to exploit
    let legitimate_user = Keypair::new();

    // Fund legitimate user's account
    ctx.send_solana_tx(&[system_instruction::transfer(
        &ctx.solana_chain.fixture.payer.pubkey(),
        &legitimate_user.pubkey(),
        u32::MAX.into(),
    )])
    .await
    .unwrap();

    // Create the target token mint
    let target_token_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(legitimate_user.pubkey(), spl_token_2022::id(), 9)
        .await;

    // Create metadata for the target token
    let (target_metadata_pda, _) =
        mpl_token_metadata::accounts::Metadata::find_pda(&target_token_mint);
    let target_metadata_ix = CreateV1Builder::new()
        .metadata(target_metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(target_token_mint, false)
        .authority(legitimate_user.pubkey())
        .update_authority(legitimate_user.pubkey(), true)
        .payer(legitimate_user.pubkey())
        .is_mutable(false)
        .name("Target Token".to_string())
        .symbol("TARGET".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[target_metadata_ix],
            &[
                &legitimate_user.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Register the target token as MintBurn type
    let target_salt = solana_sdk::keccak::hash(b"TargetToken").0;
    let target_register_ix = axelar_solana_its::instruction::register_custom_token(
        legitimate_user.pubkey(),
        target_salt,
        target_token_mint,
        token_manager::Type::MintBurn, // Using MintBurn type so handover is allowed
        spl_token_2022::id(),
        Some(legitimate_user.pubkey()),
    )
    .unwrap();

    let target_tx = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[target_register_ix],
            &[
                &legitimate_user.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Get the target token_id
    let target_token_id_event = target_tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenIdClaimed::try_from_log(log).ok())
        .unwrap();

    let target_token_id = target_token_id_event.token_id;

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
        token_manager::Type::MintBurn, // Bob also uses MintBurn type for his token
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

    // Step 3: Get the token manager PDA for the target token
    let (target_token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &target_token_id);

    // Verify the target token manager was created with MintBurn type
    let target_token_manager_data = ctx
        .solana_chain
        .fixture
        .get_account(&target_token_manager_pda, &axelar_solana_its::id())
        .await;

    let mut target_token_manager_account = target_token_manager_data.clone();
    let target_token_manager = target_token_manager_account
        .deserialize::<TokenManager>(&target_token_manager_pda)
        .unwrap();

    assert_eq!(target_token_manager.token_address, target_token_mint);
    assert_eq!(target_token_manager.ty, token_manager::Type::MintBurn);

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

    assert_msg_present_in_logs(
        tx_metadata,
        "TokenManager PDA does not match the provided Mint account",
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

    // Verify the target token's mint authority is unchanged (should still be the legitimate user)
    let target_mint_data = ctx
        .solana_chain
        .fixture
        .get_account(&target_token_mint, &spl_token_2022::id())
        .await
        .data;

    let target_mint_state = spl_token_2022::state::Mint::unpack(&target_mint_data).unwrap();
    assert_eq!(
        target_mint_state.mint_authority.unwrap(),
        legitimate_user.pubkey(),
        "Target token mint authority should still be the legitimate user (unchanged)"
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
        token_manager::Type::MintBurn,
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

    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &alice.pubkey(),
        &alice.pubkey(),
        &alice_token_mint,
        &spl_token_2022::id(),
    );

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[create_ata_ix],
            &[
                &alice.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let mint_amount = 1000u64;
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        alice_token_id,
        alice_token_mint,
        alice_ata,
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

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_fail_handover_mint_authority_for_lock_unlock_token(ctx: &mut ItsTestContext) {
    // Create a user who will try to handover mint authority for a LockUnlock token
    let user = Keypair::new();

    // Fund user's account
    ctx.send_solana_tx(&[system_instruction::transfer(
        &ctx.solana_chain.fixture.payer.pubkey(),
        &user.pubkey(),
        u32::MAX.into(),
    )])
    .await
    .unwrap();

    // Step 1: User creates a new token mint
    let user_token_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(user.pubkey(), spl_token_2022::id(), 9)
        .await;

    // Create metadata for the token
    let (metadata_pda, _) = mpl_token_metadata::accounts::Metadata::find_pda(&user_token_mint);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(user_token_mint, false)
        .authority(user.pubkey())
        .update_authority(user.pubkey(), true)
        .payer(user.pubkey())
        .is_mutable(false)
        .name("Lock Unlock Token".to_string())
        .symbol("LOCK".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[metadata_ix],
            &[
                &user.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Step 2: Register the token as LockUnlock type
    let salt = solana_sdk::keccak::hash(b"LockUnlockToken").0;
    let register_ix = axelar_solana_its::instruction::register_custom_token(
        user.pubkey(),
        salt,
        user_token_mint,
        token_manager::Type::LockUnlock, // Using LockUnlock type instead of MintBurn
        spl_token_2022::id(),
        Some(user.pubkey()),
    )
    .unwrap();

    let tx = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[register_ix],
            &[
                &user.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Get the token_id from the transaction events
    let token_id_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenIdClaimed::try_from_log(log).ok())
        .unwrap();

    let token_id = token_id_event.token_id;

    // Verify the TokenManager was created with LockUnlock type
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);

    let token_manager_data = ctx
        .solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await;

    let mut token_manager_account = token_manager_data.clone();
    let token_manager = token_manager_account
        .deserialize::<TokenManager>(&token_manager_pda)
        .unwrap();

    assert_eq!(token_manager.token_address, user_token_mint);
    assert_eq!(token_manager.ty, token_manager::Type::LockUnlock);

    // Step 3: Attempt to handover mint authority (this should fail)
    let handover_ix = axelar_solana_its::instruction::token_manager::handover_mint_authority(
        user.pubkey(),
        token_id,
        user_token_mint,
        spl_token_2022::id(),
    )
    .unwrap();

    let tx_result = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[handover_ix],
            &[
                &user.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    // Verify the transaction failed with the expected error
    let tx_metadata = tx_result.unwrap_err();
    assert_msg_present_in_logs(tx_metadata, "Invalid TokenManager type for instruction");

    // Verify the mint authority is still the user (unchanged)
    let mint_data = ctx
        .solana_chain
        .fixture
        .get_account(&user_token_mint, &spl_token_2022::id())
        .await
        .data;

    let mint_state = spl_token_2022::state::Mint::unpack(&mint_data).unwrap();
    assert_eq!(
        mint_state.mint_authority.unwrap(),
        user.pubkey(),
        "User should still be the mint authority (handover should have failed)"
    );

    // Verify user does NOT have minter role
    let (user_roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::id(),
        &token_manager_pda,
        &user.pubkey(),
    );

    let user_roles_result = ctx
        .solana_chain
        .fixture
        .try_get_account(&user_roles_pda, &axelar_solana_its::id())
        .await
        .unwrap();

    // The user might have operator role from registering the token, but shouldn't have minter role
    if let Some(account) = user_roles_result {
        let user_roles = UserRoles::<Roles>::try_from_slice(&account.data).unwrap();
        assert!(
            !user_roles.contains(Roles::MINTER),
            "User should not have minter role since handover failed"
        );
    }
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_fail_handover_mint_authority_for_native_interchain_token(ctx: &mut ItsTestContext) {
    // The deployed_interchain_token from the test context is a NativeInterchainToken
    let target_token_id = ctx.deployed_interchain_token;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (target_token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &target_token_id);

    // Get the token manager to verify it's NativeInterchainToken type
    let target_token_manager_data = ctx
        .solana_chain
        .fixture
        .get_account(&target_token_manager_pda, &axelar_solana_its::id())
        .await;

    let mut target_token_manager_account = target_token_manager_data.clone();
    let target_token_manager = target_token_manager_account
        .deserialize::<TokenManager>(&target_token_manager_pda)
        .unwrap();

    assert_eq!(
        target_token_manager.ty,
        token_manager::Type::NativeInterchainToken,
        "Test assumes deployed token is NativeInterchainToken type"
    );

    let target_token_mint = target_token_manager.token_address;

    // The deployer (payer) should be the operator of this token
    // Attempt to handover mint authority (should fail for NativeInterchainToken)
    let handover_ix = axelar_solana_its::instruction::token_manager::handover_mint_authority(
        ctx.solana_chain.fixture.payer.pubkey(),
        target_token_id,
        target_token_mint,
        spl_token_2022::id(),
    )
    .unwrap();

    let tx_metadata = ctx.send_solana_tx(&[handover_ix]).await.unwrap_err();
    assert_msg_present_in_logs(tx_metadata, "Invalid TokenManager type for instruction");

    // Verify the mint authority is still the token manager (unchanged)
    let mint_data = ctx
        .solana_chain
        .fixture
        .get_account(&target_token_mint, &spl_token_2022::id())
        .await
        .data;

    let mint_state = spl_token_2022::state::Mint::unpack(&mint_data).unwrap();
    assert_eq!(
        mint_state.mint_authority.unwrap(),
        target_token_manager_pda,
        "Token manager should still be the mint authority (handover should have failed)"
    );
}
