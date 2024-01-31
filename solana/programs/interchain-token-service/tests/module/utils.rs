use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::signature::Keypair;

pub fn program_test() -> ProgramTest {
    // Add other programs here as needed

    ProgramTest::new(
        &env!("CARGO_PKG_NAME").replace('-', "_"),
        interchain_token_service::id(),
        processor!(interchain_token_service::processor::Processor::process_instruction),
    )
}
pub struct TestFixture {
    pub banks_client: BanksClient,
    pub payer: Keypair,
}

impl TestFixture {
    pub async fn new() -> TestFixture {
        let (banks_client, payer, _recent_blockhash) = program_test().start().await;
        TestFixture {
            banks_client,
            payer,
        }
    }
}
