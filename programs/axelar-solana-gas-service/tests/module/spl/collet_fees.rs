use axelar_solana_gateway_test_fixtures::base::TestFixture;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
async fn test_collect_spl_fees(#[case] token_program_id: Pubkey) {
    // Setup the test fixture and deploy the gas service program

    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Setup a mint and mint some tokens to the payer
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let decimals = 10;
    let mint = test_fixture
        .init_new_mint(mint_authority.pubkey(), token_program_id, decimals)
        .await;
    let payer_ata = test_fixture
        .init_associated_token_account(&mint, &payer.pubkey(), &token_program_id)
        .await;
    let gas_amount = 1_000_000;

    // Setup the config_pda ATA
    let config_pda_ata = test_fixture
        .init_associated_token_account(&mint, &gas_utils.config_pda, &token_program_id)
        .await;
    test_fixture
        .mint_tokens_to(
            &mint,
            &config_pda_ata,
            &mint_authority,
            gas_amount,
            &token_program_id,
        )
        .await;

    // Fetch payer and config_pda ATA balances before
    let payer_token_before = test_fixture.get_token_account(&payer_ata).await.amount;
    let config_pda_token_before = test_fixture.get_token_account(&config_pda_ata).await.amount;

    // Create the instruction for paying gas fees with SPL tokens
    let ix = axelar_solana_gas_service::instructions::collect_spl_fees_instruction(
        &axelar_solana_gas_service::ID,
        &gas_utils.config_authority.pubkey(),
        &token_program_id,
        &mint,
        &gas_utils.config_pda,
        &config_pda_ata,
        &payer_ata,
        gas_amount,
        decimals,
    )
    .unwrap();

    // Send transaction
    test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for transaction fees
                &test_fixture.payer.insecure_clone(),
                // authority must be a signer
                &gas_utils.config_authority,
            ],
        )
        .await
        .unwrap();

    // Fetch payer and config_pda ATA balances before
    let payer_token_after = test_fixture.get_token_account(&payer_ata).await.amount;
    let config_pda_token_after = test_fixture.get_token_account(&config_pda_ata).await.amount;

    // Assert that tokens got transferred
    assert_eq!(payer_token_after, payer_token_before + gas_amount);
    assert_eq!(config_pda_token_after, config_pda_token_before - gas_amount);
}
