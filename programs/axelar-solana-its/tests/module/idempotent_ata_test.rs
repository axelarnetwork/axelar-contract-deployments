use anyhow::anyhow;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program_test::tokio;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use test_context::test_context;

use crate::ItsTestContext;

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_canonical_token_registration_with_pre_existing_ata(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let mint = Keypair::new();

    let (metaplex_metadata_pda, _) =
        mpl_token_metadata::accounts::Metadata::find_pda(&mint.pubkey());
    let create_metaplex_ix = CreateV1Builder::new()
        .metadata(metaplex_metadata_pda)
        .mint(mint.pubkey(), true)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("External Pointer Fallback".to_string())
        .symbol("EPF".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();
    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[create_metaplex_ix],
            &[
                &ctx.solana_chain.fixture.payer.insecure_clone(),
                &mint.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let token_id = axelar_solana_its::canonical_interchain_token_id(&mint.pubkey());
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);

    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &mint.pubkey(),
        &spl_token_2022::id(),
    );

    let create_ata_attack_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.solana_chain.fixture.payer.pubkey(),
            &token_manager_pda,
            &mint.pubkey(),
            &spl_token_2022::id(),
        );

    ctx.send_solana_tx(&[create_ata_attack_ix])
        .await
        .expect("Attacker should be able to create ATA");

    let ata_account_before = ctx
        .solana_chain
        .try_get_account_no_checks(&token_manager_ata)
        .await?
        .ok_or_else(|| anyhow!("ATA should exist after attacker creation"))?;

    assert_eq!(
        ata_account_before.owner,
        spl_token_2022::id(),
        "ATA should be owned by token program"
    );

    let register_ix = axelar_solana_its::instruction::register_canonical_interchain_token(
        ctx.solana_chain.fixture.payer.pubkey(),
        mint.pubkey(),
        spl_token_2022::id(),
    )?;

    let tx_result = ctx.send_solana_tx(&[register_ix]).await;

    assert!(
        tx_result.is_ok(),
        "Canonical token registration should succeed even when ATA already exists: {:?}",
        tx_result.err()
    );

    // Verify the Token Manager was created successfully
    let token_manager_account = ctx
        .solana_chain
        .try_get_account_no_checks(&token_manager_pda)
        .await?
        .ok_or_else(|| anyhow!("Token Manager should have been created"))?;

    assert_eq!(
        token_manager_account.owner,
        axelar_solana_its::id(),
        "Token Manager should be owned by ITS program"
    );

    // Verify the ATA still exists and is properly configured
    let ata_account_after = ctx
        .solana_chain
        .try_get_account_no_checks(&token_manager_ata)
        .await?
        .ok_or_else(|| anyhow!("ATA should still exist after registration"))?;

    assert_eq!(
        ata_account_after.owner,
        spl_token_2022::id(),
        "ATA should still be owned by token program"
    );

    Ok(())
}
