use solana_program_test::{processor, ProgramTest};

pub(crate) fn program_test() -> ProgramTest {
    ProgramTest::new(
        "auth_weighted",
        auth_weighted::id(),
        processor!(auth_weighted::processor::Processor::process_instruction),
    )
}
