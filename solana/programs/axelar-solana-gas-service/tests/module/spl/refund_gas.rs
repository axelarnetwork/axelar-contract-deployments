use axelar_solana_gas_service::processor::GasServiceEvent;
use axelar_solana_gas_service::processor::SplGasRefundedEvent;
use axelar_solana_gateway_test_fixtures::{base::TestFixture, gas_service::get_gas_service_events};
use gateway_event_stack::ProgramInvocationState;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
async fn test_refund_spl_fees(#[case] token_program_id: Pubkey) {
    // Setup the test fixture and deploy the gas service program

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
    let log_index = 42;
    let ix = axelar_solana_gas_service::instructions::refund_spl_fees_instruction(
        &axelar_solana_gas_service::ID,
        &gas_utils.config_authority.pubkey(),
        &token_program_id,
        &mint,
        &gas_utils.config_pda,
        &config_pda_ata,
        &receiver_ata,
        tx_hash,
        log_index,
        gas_amount,
        decimals,
    )
    .unwrap();

    // Send transaction
    let res = test_fixture
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

    // Assert event
    let emitted_events = get_gas_service_events(&res)
        .into_iter()
        .next()
        .expect("No events emitted");
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event");
    };

    let [(_, GasServiceEvent::SplGasRefunded(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event sequence");
    };

    assert_eq!(
        emitted_event,
        &SplGasRefundedEvent {
            config_pda_ata,
            mint,
            token_program_id,
            tx_hash,
            config_pda: gas_utils.config_pda,
            log_index,
            receiver: receiver_ata,
            fees: gas_amount,
        }
    );

    // Fetch payer and config_pda ATA balances before
    let payer_token_after = test_fixture.get_token_account(&receiver_ata).await.amount;
    let config_pda_token_after = test_fixture.get_token_account(&config_pda_ata).await.amount;

    // Assert that tokens got transferred
    assert_eq!(payer_token_after, payer_token_before + gas_amount);
    assert_eq!(config_pda_token_after, config_pda_token_before - gas_amount);
}
