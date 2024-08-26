use test_fixtures::test_setup::{SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata};

mod initialize;
mod send_to_gateway;
mod validate_message;

pub async fn program_test() -> SolanaAxelarIntegrationMetadata {
    SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(vec![(
            "axelar_solana_memo_program.so".into(),
            axelar_solana_memo_program::id(),
        )])
        .build()
        .setup()
        .await
}
