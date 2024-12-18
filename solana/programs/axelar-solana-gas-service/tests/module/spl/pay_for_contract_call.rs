use axelar_solana_gas_service::processor::GasServiceEvent;
use axelar_solana_gas_service::processor::SplGasPaidForContractCallEvent;
use axelar_solana_gateway_test_fixtures::{base::TestFixture, gas_service::get_gas_service_events};
use gateway_event_stack::ProgramInvocationState;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
async fn test_pay_spl_for_contract_call(#[case] token_program_id: Pubkey) {
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
    test_fixture
        .mint_tokens_to(
            &mint,
            &payer_ata,
            &mint_authority,
            gas_amount,
            &token_program_id,
        )
        .await;

    // Setup the config_pda ATA
    let config_pda_ata = test_fixture
        .init_associated_token_account(&mint, &gas_utils.config_pda, &token_program_id)
        .await;

    // Fetch payer and config_pda ATA balances before
    let payer_token_before = test_fixture.get_token_account(&payer_ata).await.amount;
    let config_pda_token_before = test_fixture.get_token_account(&config_pda_ata).await.amount;

    // Prepare args
    let refund_address = Pubkey::new_unique();
    let destination_chain = "ethereum".to_owned();
    let destination_addr = "0x destination addr 123".to_owned();
    let payload_hash = [42; 32];
    let params = b"hello 123321".to_vec();

    dbg!(&config_pda_ata);
    // Create the instruction for paying gas fees with SPL tokens
    let ix = axelar_solana_gas_service::instructions::pay_spl_for_contract_call_instruction(
        &axelar_solana_gas_service::ID,
        &payer.pubkey(),
        &payer_ata,
        &gas_utils.config_pda,
        &config_pda_ata,
        &mint,
        &token_program_id,
        destination_chain.clone(),
        destination_addr.clone(),
        payload_hash,
        refund_address,
        params.clone(),
        gas_amount,
        &[],
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
                // payer signs to transfer tokens
                &payer,
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

    let [(_, GasServiceEvent::SplGasPaidForContractCall(emitted_event))] = vec_events.as_slice()
    else {
        panic!("unexpected event sequence");
    };

    assert_eq!(
        emitted_event,
        &SplGasPaidForContractCallEvent {
            config_pda: gas_utils.config_pda,
            config_pda_ata,
            mint,
            token_program_id,
            destination_chain,
            destination_address: destination_addr,
            payload_hash,
            refund_address,
            params,
            gas_fee_amount: gas_amount
        }
    );

    // Fetch payer and config_pda ATA balances before
    let payer_token_after = test_fixture.get_token_account(&payer_ata).await.amount;
    let config_pda_token_after = test_fixture.get_token_account(&config_pda_ata).await.amount;

    // Assert that tokens got transferred
    assert_eq!(payer_token_after, payer_token_before - gas_amount);
    assert_eq!(config_pda_token_after, config_pda_token_before + gas_amount);
}
