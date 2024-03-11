use gateway::state::GatewayConfig;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::Bytes32;
use solana_program::pubkey::Pubkey;
use test_fixtures::execute_data::{create_signer_with_weight, TestSigner};
use test_fixtures::test_setup::TestFixture;
use token_manager::get_token_manager_account;

mod deploy_interchain_token;
mod deploy_remote_interchain_token;
mod deploy_remote_token_manager;
mod deploy_token_manager;
mod give_token;
mod interchain_transfer;
mod intialize;
mod remote_interchain_transfer;
mod take_token;

use solana_program_test::{processor, ProgramTest};

pub fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::new(
        "interchain_token_service",
        interchain_token_service::id(),
        processor!(interchain_token_service::processor::Processor::process_instruction),
    );
    pt.add_program(
        "gmp_gateway",
        gateway::id(),
        processor!(gateway::processor::Processor::process_instruction),
    );
    pt.add_program(
        "gas_service",
        gas_service::id(),
        processor!(gas_service::processor::Processor::process_instruction),
    );
    pt.add_program(
        "token_manager",
        token_manager::id(),
        processor!(token_manager::processor::Processor::process_instruction),
    );
    pt.add_program(
        "account_group",
        account_group::id(),
        processor!(account_group::processor::Processor::process_instruction),
    );
    pt.add_program(
        "interchain_address_tracker",
        interchain_address_tracker::id(),
        processor!(account_group::processor::Processor::process_instruction),
    );

    pt
}

/// Setup valid Gateway + GasService + ITSRoot + TokenManager groups
async fn setup_its_root_fixture() -> (
    TestFixture,
    Pubkey,
    Pubkey,
    Pubkey,
    test_fixtures::test_setup::ITSTokenHandlerGroups,
    Pubkey,
    Vec<TestSigner>,
) {
    let mut fixture = TestFixture::new(program_test()).await;
    let gas_service_root_pda = fixture.init_gas_service().await;
    let token_id = Bytes32(keccak256("random-token-id"));
    let gateway_operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let init_operator = Pubkey::from([0; 32]);
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::new(
            0,
            fixture.init_operators_and_epochs(&gateway_operators),
        ))
        .await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let its_token_manager_permission_groups = fixture
        .derive_token_manager_permission_groups(
            &token_id,
            &interchain_token_service_root_pda,
            &interchain_token_service_root_pda,
            &init_operator,
        )
        .await;
    let token_manager_root_pda_pubkey = get_token_manager_account(
        &its_token_manager_permission_groups.operator_group.group_pda,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda,
        &interchain_token_service_root_pda,
    );
    (
        fixture,
        gas_service_root_pda,
        gateway_root_pda,
        interchain_token_service_root_pda,
        its_token_manager_permission_groups,
        token_manager_root_pda_pubkey,
        gateway_operators,
    )
}
