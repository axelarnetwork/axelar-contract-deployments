# Solana-Axelar Interoperability

This repository contains the integration work between Solana and Axelar, enabling seamless cross-chain communication. The project includes General Message Passing (GMP) contracts and other Axelar core components.

## Table of Contents

- [Repository contents](#repository-contents)
  - [Solana contracts](#solana-contracts)
    - [Utility crates](#utility-crates)
  - [EVM Smart contracts](#evm-smart-contracts)
  - [Related repositories](#related-repositories)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)

## Repository contents

![image](https://github.com/user-attachments/assets/88008f1c-4096-4248-87b2-128b65cb8e41)

The Solana-Axelar integration contains on-chain and off-chain components.


### Development setup:

1. [install rust](https://www.rust-lang.org/tools/install)
2. [install solana tool suite](https://docs.solanalabs.com/cli/install)
3. [install foundry & build EVM contracts](evm-contracts/README.md)

```bash
# list all available commands
cargo xtask --help
```
### Solana contracts

- [**Gateway**](programs/axelar-solana-gateway/README.md): The core contract responsible for authenticating GMP messages.
- [**Gas Service**](programs/axelar-solana-gas-service/README.md): Used for gas payments for the relayer.
- [**Interchain Token Service**](programs/axelar-solana-its/README.md): Bridge tokens between chains.
- [**Multicall**](programs/axelar-solana-multicall): Execute multiple actions from a single GMP message.
- [**Governance**](programs/axelar-solana-governance/README.md): The governing entity over on-chain programs, responsible for program upgrades.
- [**Memo**](programs/axelar-solana-memo-program): An example program that sends and receives GMP messages.


#### Utility crates
- [**Axelar Solana Encoding**](solana/crates/axelar-solana-encoding/README.md): Encoding used by the Multisig Prover to encode the data in a way that the relayer & the Solana Gateway can interpret.

### EVM Smart Contracts
- [**Axelar Memo**](evm-contracts/src/AxelarMemo.sol): A counterpart of the `axelar-solana-memo` program that acts as an example program used to send GMP messages back and forth Solana.
- [**Axelar Solana Multi Call**](evm-contracts/src/AxelarSolanaMultiCall.sol): An example contract used to showcase how to compose Multicall payloads for Solana.
- [**Solana Gateway Payload**](evm-contracts/src/ExampleEncoder.sol): A Solditiy library that can create Solana-specific GMP payloads.


## Related Repositories

- [**Solana Relayer**](https://github.com/eigerco/axelar-solana-relayer): The off-chain entity that will route your messages to and from Solana.
- [**Relayer Core**](https://github.com/eigerco/axelar-relayer-core): All Axelar-related relayer infrastructure. Used as a core building block for the Solana Relayer. The Axelar-Starknet and Axlelar-Aleo relayers also use it.
- [**Multisig Prover**](https://github.com/eigerco/axelar-amplifier/tree/add-multisig-prover-sol-logic/contracts/multisig-prover): The entity on the Axelar chain that is responsible for encoding the data for the Relayer and the Solana Gateway
- [**Utility Scripts**](https://github.com/eigerco/axelar-contract-deployments): Contract deployment scripts and resources for Axelar.


## Getting Started

### Prerequisites

- [Solana CLI (for running tests during development)](https://solana.com/docs/intro/installation)
- [Foundry (for running e2e tests, GMP examples between Solana and an EVM chain)](https://book.getfoundry.sh/getting-started/installation)

### Installation

```bash
git clone git@github.com:eigerco/axelar-amplifier-solana.git
cd solana
cargo xtask test
```

## About [Eiger](https://www.eiger.co)

We are engineers. We contribute to various ecosystems by building low-level implementations and core components. We work on several Axelar and Solana projects and connecting these two is a fundamental goal to achieve cross-chain execution.

Contact us at hello@eiger.co
Follow us on [X/Twitter](https://x.com/eiger_co)
