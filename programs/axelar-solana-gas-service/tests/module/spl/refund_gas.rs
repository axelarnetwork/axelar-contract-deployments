use axelar_solana_gateway_test_fixtures::base::TestFixture;
use event_cpi_test_utils::assert_event_cpi;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
async fn test_refund_spl_fees(#[case] token_program_id: Pubkey) {
    // Setup the test fixture and deploy the gas service program

    use axelar_solana_gas_service::events::SplGasRefundedEvent;

    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Setup a mint and mint some tokens to the payer
    let receiver = Keypair::new();
    let mint_authority = Keypair::new();
    let decimals = 10;
    let mint = test_fixture
        .init_new_mint(mint_authority.pubkey(), token_program_id, decimals)
        .await;
    let receiver_ata = test_fixture
        .init_associated_token_account(&mint, &receiver.pubkey(), &token_program_id)
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
    let payer_token_before = test_fixture.get_token_account(&receiver_ata).await.amount;
    let config_pda_token_before = test_fixture.get_token_account(&config_pda_ata).await.amount;

    // Create the instruction for paying gas fees with SPL tokens
    let tx_hash = [132; 64];
    let ix_index = 1;
    let event_ix_index = 2;
    let ix = axelar_solana_gas_service::instructions::refund_spl_fees_instruction(
        &gas_utils.operator.pubkey(),
        &token_program_id,
        &mint,
        &receiver_ata,
        tx_hash,
        ix_index,
        event_ix_index,
        gas_amount,
        decimals,
    )
    .unwrap();

    // First simulate to check events
    let simulation_result = test_fixture
        .simulate_tx_with_custom_signers(
            &[ix.clone()],
            &[
                // pays for transaction fees
                &test_fixture.payer.insecure_clone(),
                // operator must be a signer
                &gas_utils.operator,
            ],
        )
        .await
        .unwrap();

    // Assert event emitted
    let inner_ixs = simulation_result
        .simulation_details
        .unwrap()
        .inner_instructions
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    assert!(!inner_ixs.is_empty());

    let expected_event = SplGasRefundedEvent {
        config_pda_ata,
        mint,
        token_program_id,
        tx_hash,
        config_pda: gas_utils.config_pda,
        ix_index,
        event_ix_index,
        receiver: receiver_ata,
        fees: gas_amount,
    };

    assert_event_cpi(&expected_event, &inner_ixs);

    // Execute the transaction
    let _res = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for transaction fees
                &test_fixture.payer.insecure_clone(),
                // operator must be a signer
                &gas_utils.operator,
            ],
        )
        .await
        .unwrap();

    // Fetch payer and config_pda ATA balances before
    let payer_token_after = test_fixture.get_token_account(&receiver_ata).await.amount;
    let config_pda_token_after = test_fixture.get_token_account(&config_pda_ata).await.amount;

    // Assert that tokens got transferred
    assert_eq!(payer_token_after, payer_token_before + gas_amount);
    assert_eq!(config_pda_token_after, config_pda_token_before - gas_amount);
}
