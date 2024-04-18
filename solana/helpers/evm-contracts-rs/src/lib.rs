#![warn(missing_docs, unreachable_pub, unused_crate_dependencies)]
#![deny(unused_must_use, rust_2018_idioms)]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))
))]

//! # `ethers-gen`
//! Contains all the generated bindings for the EVM contracts.

pub use ethers;

/// The `contracts` module contains all the generated bindings for the EVM
/// contracts.
#[allow(clippy::all, missing_docs)]
pub mod contracts {
    use ethers::contract::abigen;

    abigen!(
        ExampleEncoder,
        "../../../evm-contracts/out/ExampleEncoder.sol/ExampleSolanaGatewayEncoder.json"
    );
}
