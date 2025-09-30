#[allow(clippy::use_debug, clippy::indexing_slicing, clippy::print_stderr)]
pub fn contains_event_cpi<E: event_cpi::CpiEvent + std::fmt::Debug + PartialEq>(
    event: &E,
    ixs: &[solana_sdk::inner_instruction::InnerInstruction],
) -> bool {
    let mut found = false;
    for ix in ixs {
        let data = &ix.instruction.data;

        // Check if we have enough data for the event discriminator
        if data.len() < 16 {
            continue;
        }

        let ev_disc = &data[0..8];

        // Check if the instruction is an event-cpi instruction
        if ev_disc != event_cpi::EVENT_IX_TAG_LE {
            continue;
        }

        // Check if the event discriminator matches
        let disc = &data[8..16];
        if disc != E::DISCRIMINATOR {
            continue;
        }

        let event_data = &data[16..];
        let decoded_event = E::try_from_slice(event_data).unwrap();

        if decoded_event == *event {
            found = true;
            break;
        }

        eprintln!("Found event with correct discriminator, incorrect data");
        eprintln!("Decoded event: {decoded_event:#?}");
        eprintln!("Expected event: {event:#?}");
    }

    found
}

pub fn assert_event_cpi<E: event_cpi::CpiEvent + std::fmt::Debug + PartialEq>(
    event: &E,
    ixs: &[solana_sdk::inner_instruction::InnerInstruction],
) {
    let found = contains_event_cpi(event, ixs);
    assert!(found, "Event not found in inner instructions");
}
