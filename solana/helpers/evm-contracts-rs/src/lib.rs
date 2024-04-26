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
    pub mod example_encoder {
        include!(concat!(env!("OUT_DIR"), "/ExampleEncoder.rs"));
    }

    pub mod axelar_memo {
        include!(concat!(env!("OUT_DIR"), "/AxelarMemo.rs"));
    }

    pub mod axelar_gateway {
        include!(concat!(env!("OUT_DIR"), "/AxelarGateway.rs"));
    }

    pub mod axelar_auth_weighted {
        include!(concat!(env!("OUT_DIR"), "/AxelarAuthWeighted.rs"));
    }
}
