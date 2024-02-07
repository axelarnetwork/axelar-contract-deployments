use account_group::instruction::GroupId;
use account_group::{get_permission_account, get_permission_group_account};
use solana_program::clock::Clock;
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use token_manager::CalculatedEpoch;

pub fn program_test() -> ProgramTest {
    let mut program = ProgramTest::new(
        &env!("CARGO_PKG_NAME").replace('-', "_"),
        token_manager::id(),
        processor!(token_manager::processor::Processor::process_instruction),
    );

    program.add_program(
        "account_group",
        account_group::id(),
        processor!(account_group::processor::Processor::process_instruction),
    );

    program
}
pub struct TestFixture {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub service_program_pda: Keypair,
    pub operator_repr: OperatorRepr,
    pub flow_repr: OperatorRepr,
}

pub struct OperatorRepr {
    pub operator_group_pda: Pubkey,
    pub init_operator_pda_acc: Pubkey,
    pub operator: Keypair,
    pub operator_group_id: GroupId,
}

impl TestFixture {
    /// Crete a new test fixture with a new registered chain account already
    /// created.
    pub async fn new() -> TestFixture {
        let service_program_pda = Keypair::new();
        let operator = Keypair::new();
        let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
        let operator_group_id = GroupId::new("test-op-group-id");
        let flow_group_id = GroupId::new("test-flow-group-id");

        let (operator_group_pda, init_operator_pda_acc) = operator_group(
            operator_group_id.clone(),
            &operator,
            &mut banks_client,
            &payer,
            recent_blockhash,
        )
        .await;
        let (flow_group_pda, init_flow_pda_acc) = operator_group(
            flow_group_id.clone(),
            &operator,
            &mut banks_client,
            &payer,
            recent_blockhash,
        )
        .await;
        TestFixture {
            banks_client,
            service_program_pda,
            payer,
            flow_repr: OperatorRepr {
                operator: operator.insecure_clone(),
                operator_group_pda: flow_group_pda,
                init_operator_pda_acc: init_flow_pda_acc,
                operator_group_id: flow_group_id,
            },
            operator_repr: OperatorRepr {
                operator_group_pda,
                init_operator_pda_acc,
                operator_group_id,
                operator: operator.insecure_clone(),
            },
        }
    }

    pub async fn post_setup(mut self, flow_limit: u64) -> (Self, Pubkey) {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();

        let token_manager_pda = token_manager::get_token_manager_account(
            &self.operator_repr.operator_group_pda,
            &self.flow_repr.operator_group_pda,
            &self.service_program_pda.pubkey(),
        );
        let clock = self.banks_client.get_sysvar::<Clock>().await.unwrap();
        let block_timestamp = clock.unix_timestamp;

        let _token_flow_pda = token_manager::get_token_flow_account(
            &token_manager_pda,
            CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
        );

        let ix = token_manager::instruction::build_setup_instruction(
            &self.payer.pubkey(),
            &token_manager_pda,
            &self.operator_repr.operator_group_pda,
            &self.operator_repr.init_operator_pda_acc,
            &self.operator_repr.operator.pubkey(),
            &self.flow_repr.operator_group_pda,
            &self.flow_repr.init_operator_pda_acc,
            &self.flow_repr.operator.pubkey(),
            &self.service_program_pda.pubkey(),
            token_manager::instruction::Setup { flow_limit },
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[
                &self.payer,
                &self.operator_repr.operator,
                &self.flow_repr.operator,
            ],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        (self, token_manager_pda)
    }
}

async fn operator_group(
    operator_group_id: GroupId,
    operator: &Keypair,
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: solana_program::hash::Hash,
) -> (Pubkey, Pubkey) {
    let operator_group_pda = get_permission_group_account(&operator_group_id);
    let init_operator_pda_acc = get_permission_account(&operator_group_pda, &operator.pubkey());

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
        operator_group_id,
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer, operator],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
    (operator_group_pda, init_operator_pda_acc)
}
