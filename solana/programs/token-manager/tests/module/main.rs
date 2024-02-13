use solana_program_test::{processor, ProgramTest};
mod add_flow;
mod set_flow_limit;
mod setup;

pub fn program_test() -> ProgramTest {
    // Add other programs here as needed
    let mut pt = ProgramTest::new(
        "gmp_gateway",
        gateway::id(),
        processor!(gateway::processor::Processor::process_instruction),
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

    pt
}
