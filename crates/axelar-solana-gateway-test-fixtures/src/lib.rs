//! Test utilities for the Solana Gateway
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::multiple_inherent_impl)]
#![allow(clippy::wildcard_enum_match_arm)]
#![allow(clippy::unimplemented)]
#![allow(deprecated)]

pub mod base;
pub mod gas_service;
pub mod gateway;
pub mod test_signer;

pub use gateway::{SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata};
use solana_program_test::BanksTransactionResultWithMetadata;

pub fn assert_msg_present_in_logs(res: BanksTransactionResultWithMetadata, msg: &str) {
    assert!(
        res.metadata
            .unwrap()
            .log_messages
            .into_iter()
            .any(|x| x.contains(msg)),
        "Expected error message not found!"
    );
}

pub fn find_event_cpi<E: event_cpi::CpiEvent + std::fmt::Debug + PartialEq>(
    event: &E,
    ixs: &[solana_sdk::inner_instruction::InnerInstruction],
) -> bool {
    let mut found = false;
    for ix in ixs {
        // TODO check program id

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

        if decoded_event != *event {
            println!("Found event with correct discriminator, incorrect data");
            println!("Decoded event: {:#?}", decoded_event);
            println!("Expected event: {:#?}", event);
            continue;
        } else {
            found = true;
            break;
        }
    }

    found
}

// TODO move to another test utils crate
pub fn assert_event_cpi<E: event_cpi::CpiEvent + std::fmt::Debug + PartialEq>(
    event: &E,
    ixs: &[solana_sdk::inner_instruction::InnerInstruction],
) {
    let found = find_event_cpi(event, ixs);
    assert!(found, "Event not found in inner instructions");
}
