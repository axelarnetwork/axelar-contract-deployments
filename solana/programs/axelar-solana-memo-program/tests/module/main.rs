use solana_program_test::{processor, ProgramTest};

mod validate_contract_call;

pub fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::new(
        "gmp_gateway",
        gateway::id(),
        processor!(gateway::processor::Processor::process_instruction),
    );

    pt.add_program(
        "axelar_solana_memo_program",
        axelar_solana_memo_program::id(),
        processor!(axelar_solana_memo_program::processor::process_instruction),
    );

    pt
}
