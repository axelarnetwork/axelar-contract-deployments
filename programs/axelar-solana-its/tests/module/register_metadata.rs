use axelar_solana_gateway_test_fixtures::assert_msg_present_in_logs;
use solana_program_test::tokio;
use solana_sdk::{
    instruction::AccountMeta, program_pack::Pack, signature::Keypair, signer::Signer,
};
use test_context::test_context;

use crate::ItsTestContext;

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_register_metadata_invalid_mint(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let fake_mint = Keypair::new();
    let mint_space = spl_token_2022::state::Mint::LEN;
    let mint_rent = ctx.solana_chain.fixture.get_rent(mint_space).await;

    #[allow(clippy::disallowed_methods)]
    let create_mint_account_ix = solana_sdk::system_instruction::create_account(
        &ctx.solana_wallet,
        &fake_mint.pubkey(),
        mint_rent,
        mint_space as u64,
        &ctx.solana_wallet,
    );

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[create_mint_account_ix],
            &[
                ctx.solana_chain.fixture.payer.insecure_clone(),
                fake_mint.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let custom_solana_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let mut register_metadata = axelar_solana_its::instruction::register_token_metadata(
        ctx.solana_wallet,
        custom_solana_token,
        spl_token_2022::id(),
        0,
    )?;

    register_metadata.accounts[1] = AccountMeta::new_readonly(fake_mint.pubkey(), false);

    let tx = ctx.send_solana_tx(&[register_metadata]).await;
    assert!(tx.is_err());
    assert_msg_present_in_logs(tx.unwrap_err(), "Invalid mint account");

    Ok(())
}
