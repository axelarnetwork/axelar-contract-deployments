mod deploy_interchain_token;
mod deploy_remote_token_manager;
mod deploy_token_manager;
mod give_token;
mod interchain_transfer;
mod intialize;
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
