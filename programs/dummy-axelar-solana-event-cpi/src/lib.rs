#![warn(missing_docs, unreachable_pub)]
#![deny(unused_must_use, rust_2018_idioms)]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))
))]

//! Simple demo program that uses the event_cpi helper crates

mod entrypoint;
pub mod instruction;
pub mod processor;
use program_utils::ensure_single_feature;
pub use solana_program;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
solana_program::declare_id!("cpiPJFxP6H6bjEKpUSJ4KC7C4dKAfNE3xWrTpJBKDwN");

#[cfg(feature = "stagenet")]
solana_program::declare_id!("cpidp6koMvx6Bneq1BJvtf7YEKNQDiNmnMFfE6fP691");

#[cfg(feature = "testnet")]
solana_program::declare_id!("cpigw1yvm5Q4MVzsTyyz7MdzMUtB1wZC8HeH2ZJABh2");

#[cfg(feature = "mainnet")]
solana_program::declare_id!("cpi1111111111111111111111111111111111111111");
