use solana_program_test::{processor, ProgramTest};

// TODO write test for trying to re-initialize an approved message (approving it
//      twice)
// TODO write test for approving a message that's already been executed

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gmp_gateway",
        gmp_gateway::id(),
        processor!(gmp_gateway::processor::Processor::process_instruction),
    )
}
