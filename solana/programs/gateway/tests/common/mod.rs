use solana_program_test::{processor, ProgramTest};
pub mod fixtures;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gateway",
        gateway::id(),
        processor!(gateway::processor::Processor::process_instruction),
    )
}
