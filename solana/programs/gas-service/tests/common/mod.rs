use solana_program_test::{processor, ProgramTest};

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gas_service",
        gas_service::id(),
        processor!(gas_service::processor::Processor::process_instruction),
    )
}
