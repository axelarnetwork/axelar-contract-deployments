use interchain_address_tracker::get_associated_chain_address;
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        &env!("CARGO_PKG_NAME").replace('-', "_"),
        interchain_address_tracker::id(),
        processor!(interchain_address_tracker::processor::Processor::process_instruction),
    )
}

pub struct TestFixture {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub owner: Keypair,
    pub associated_chain_address: Pubkey,
    pub chain_name: String,
}

impl TestFixture {
    /// Crete a new test fixture with a new registered chain account already
    /// created.
    pub async fn new() -> TestFixture {
        let owner = Keypair::new();
        let associated_chain_address = get_associated_chain_address(&owner.pubkey());
        let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

        let chain_name = "MyChainABC".to_string();

        let ix =
            interchain_address_tracker::instruction::build_create_registered_chain_instruction(
                &payer.pubkey(),
                &associated_chain_address,
                &owner.pubkey(),
                chain_name.clone(),
            )
            .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer, &owner],
            recent_blockhash,
        );
        banks_client.process_transaction(transaction).await.unwrap();

        TestFixture {
            banks_client,
            payer,
            owner,
            associated_chain_address,
            chain_name,
        }
    }
}
