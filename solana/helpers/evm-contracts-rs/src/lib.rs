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

    pub mod axelar_amplifier_gateway {
        include!(concat!(env!("OUT_DIR"), "/AxelarAmplifierGateway.rs"));
    }

    pub mod axelar_amplifier_gateway_proxy {
        include!(concat!(env!("OUT_DIR"), "/AxelarAmplifierGatewayProxy.rs"));
    }

    pub mod axelar_solana_multicall {
        include!(concat!(env!("OUT_DIR"), "/AxelarSolanaMultiCall.rs"));
    }

    pub mod interchain_token_service {
        include!(concat!(env!("OUT_DIR"), "/InterchainTokenService.rs"));
    }

    pub mod interchain_token_factory {
        include!(concat!(env!("OUT_DIR"), "/InterchainTokenFactory.rs"));
    }

    pub mod test_canonical_token {
        include!(concat!(env!("OUT_DIR"), "/TestCanonicalToken.rs"));
    }

    pub mod gateway_caller {
        include!(concat!(env!("OUT_DIR"), "/GatewayCaller.rs"));
    }

    pub mod interchain_token_deployer {
        include!(concat!(env!("OUT_DIR"), "/InterchainTokenDeployer.rs"));
    }

    pub mod interchain_token {
        include!(concat!(env!("OUT_DIR"), "/InterchainToken.rs"));
    }

    pub mod token_handler {
        include!(concat!(env!("OUT_DIR"), "/TokenHandler.rs"));
    }

    pub mod token_manager_deployer {
        include!(concat!(env!("OUT_DIR"), "/TokenManagerDeployer.rs"));
    }

    pub mod token_manager {
        include!(concat!(env!("OUT_DIR"), "/TokenManager.rs"));
    }

    pub mod axelar_gas_service {
        include!(concat!(env!("OUT_DIR"), "/AxelarGasService.rs"));
    }

    pub mod axelar_create3_deployer {
        include!(concat!(env!("OUT_DIR"), "/Create3Deployer.rs"));
    }

    pub mod interchain_proxy {
        include!(concat!(env!("OUT_DIR"), "/InterchainProxy.rs"));
    }

    pub mod axelar_auth_weighted {
        include!(concat!(env!("OUT_DIR"), "/AxelarAuthWeighted.rs"));
    }

    pub mod axelar_gateway {
        include!(concat!(env!("OUT_DIR"), "/AxelarGateway.rs"));
    }
}
