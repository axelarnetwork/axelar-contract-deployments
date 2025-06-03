use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;

// Test a solana account cannot be initialized if already has lamports (exploratory test)
#[tokio::test]
async fn test_cannot_create_pda_with_previous_lamports() {
    let mut program_test = program_test().await;

    // Ask the dummy program to generate a new PDA account
    let (create_pda_ix, (key, _)) = dummy_axelar_solana_gateway::instructions::create_raw_pda(
        &program_test.fixture.payer.pubkey(),
    );

    // We transfer 1 lamport to the PDA account so it cannot be created
    let transfer_lamports_ix =
        system_instruction::transfer(&program_test.fixture.payer.pubkey(), &key, 1_000_000);

    // Execute both instructions in a single transaction. It should fail because the PDA account already has lamports.
    program_test
        .fixture
        .send_tx(&[transfer_lamports_ix, create_pda_ix])
        .await.err().unwrap().find_log("already in use")
        .expect("We expected the solana runtime to stop PDA creation when there are lamports in the account");
}

#[tokio::test]
async fn test_can_create_pda_with_previous_lamports_using_enhanced_function() {
    let mut program_test = program_test().await;

    // Ask the dummy program to generate a new PDA account
    let (create_pda_ix, (key, _)) =
        dummy_axelar_solana_gateway::instructions::create_pda(&program_test.fixture.payer.pubkey());

    // We transfer lamports to the PDA account so it cannot be created from normal create-FUNCTION
    let transfer_lamports_ix =
        system_instruction::transfer(&program_test.fixture.payer.pubkey(), &key, 1_000_000);

    // Execute both instructions in a single transaction. It should pass even if the PDA account already has lamports.
    // This also ensures there's enough rent to initialize the PDA account.
    program_test
        .fixture
        .send_tx(&[transfer_lamports_ix, create_pda_ix])
        .await.expect("We expect the init_pda_v2 function to handle the case when the PDA account already has lamports");
}

async fn program_test() -> SolanaAxelarIntegrationMetadata {
    SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(vec![(
            "dummy_axelar_solana_gateway.so".into(),
            dummy_axelar_solana_gateway::id(),
        )])
        .build()
        .setup()
        .await
}
