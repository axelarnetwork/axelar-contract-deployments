#[allow(clippy::use_debug, clippy::indexing_slicing, clippy::print_stderr)]
pub fn contains_event_cpi<E: event_cpi::CpiEvent + std::fmt::Debug + PartialEq>(
    event: &E,
    ixs: &[solana_sdk::inner_instruction::InnerInstruction],
) -> bool {
    let mut found = false;
    for ix in ixs {
        let data = &ix.instruction.data;

        if !event_cpi_matches::<E>(data) {
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

#[allow(clippy::indexing_slicing)]
pub fn get_first_event_cpi_occurrence<E: event_cpi::CpiEvent>(
    ixs: &[solana_sdk::inner_instruction::InnerInstruction],
) -> Option<E> {
    for ix in ixs {
        let data = &ix.instruction.data;

        if !event_cpi_matches::<E>(data) {
            continue;
        }

        let event_data = &data[16..];
        if let Ok(decoded_event) = E::try_from_slice(event_data) {
            return Some(decoded_event);
        }
    }

    None
}

pub fn assert_event_cpi<E: event_cpi::CpiEvent + std::fmt::Debug + PartialEq>(
    event: &E,
    ixs: &[solana_sdk::inner_instruction::InnerInstruction],
) {
    let found = contains_event_cpi(event, ixs);

    // TODO print event name/details
    assert!(found, "Event not found in inner instructions");
}

#[allow(
    clippy::use_debug,
    clippy::indexing_slicing,
    clippy::print_stderr,
    clippy::missing_asserts_for_indexing
)]
fn event_cpi_matches<E: event_cpi::CpiEvent>(data: &[u8]) -> bool {
    // Check if we have enough data for the event discriminator
    if data.len() < 16 {
        return false;
    }

    let ev_disc = &data[0..8];

    // Check if the instruction is an event-cpi instruction
    if ev_disc != event_cpi::EVENT_IX_TAG_LE {
        return false;
    }

    // Check if the event discriminator matches the target event
    let disc = &data[8..16];
    if disc != E::DISCRIMINATOR {
        return false;
    }

    true
}
