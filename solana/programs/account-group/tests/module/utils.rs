use account_group::instruction::GroupId;
use account_group::{get_permission_account, get_permission_group_account};
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        &env!("CARGO_PKG_NAME").replace('-', "_"),
        account_group::id(),
        processor!(account_group::processor::Processor::process_instruction),
    )
}
pub struct TestFixture {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub init_operator: Keypair,
    pub init_operator_pda_acc: Pubkey,
    pub operator_group_pda: Pubkey,
    pub operator_group_id: GroupId,
}

impl TestFixture {
    /// Crete a new test fixture with a new registered chain account already
    /// created.
    pub async fn new() -> TestFixture {
        let operator = Keypair::new();
        let operator_group_id = GroupId::new("test-operation-chain-id");
        let operator_group_pda = get_permission_group_account(&operator_group_id);
        let init_operator_pda_acc = get_permission_account(&operator_group_pda, &operator.pubkey());
        let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

        // Associated account does not exist
        assert_eq!(
            banks_client
                .get_account(operator_group_pda)
                .await
                .expect("get_account"),
            None,
        );
        assert_eq!(
            banks_client
                .get_account(init_operator_pda_acc)
                .await
                .expect("get_account"),
            None,
        );

        let ix = account_group::instruction::build_setup_permission_group_instruction(
            &payer.pubkey(),
            &operator_group_pda,
            &init_operator_pda_acc,
            &operator.pubkey(),
            operator_group_id.clone(),
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer, &operator],
            recent_blockhash,
        );
        banks_client.process_transaction(transaction).await.unwrap();
        TestFixture {
            banks_client,
            payer,
            init_operator: operator,
            init_operator_pda_acc,
            operator_group_id: operator_group_id.clone(),
            operator_group_pda,
        }
    }
}
