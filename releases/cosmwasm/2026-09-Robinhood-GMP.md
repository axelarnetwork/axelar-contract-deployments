# Robinhood CosmWasm GMP

| Network | Deployment status                                                                                                                | Date           |
| ------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| Testnet | Contracts deployed; configuration recorded in [PR #1418](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1418) | September 2026 |
| Mainnet | TBD                                                                                                                              | TBD            |

## Background

This release records the Amplifier contracts connecting Axelar testnet to [Robinhood EVM GMP](../evm/2026-09-Robinhood-GMP.md). The deployment is recorded under `axelar.contracts` in [testnet.json](../../axelar-chains-config/info/testnet.json).

## Testnet contracts

| Contract       | Version                                                                                        | Code ID | Address                                                             |
| -------------- | ---------------------------------------------------------------------------------------------- | ------- | ------------------------------------------------------------------- |
| Gateway        | [1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/gateway-v1.1.1)         | `24`    | `axelar1k4jsc5tf4tgdqupzj3a89a9glx63s9vuzrypfsc9qgmkmlymw8hss80c93` |
| VotingVerifier | [2.0.2](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v2.0.2) | `87`    | `axelar1995v0ysau2cqqedjw2zh5luqlfpnn49hrhtdyhp9lxf72c765upsgupqld` |
| MultisigProver | [1.2.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-prover-v1.2.1) | `85`    | `axelar1r5yr5u8c6gdy75k3kffletp3clz5cg96e8h0sv8t5gew83f43z8q8dpvzz` |

The three versions were checked against each deployed contract's CW2 `contract_info` storage on September 29, 2026.

| Parameter                      | Value                                                                |
| ------------------------------ | -------------------------------------------------------------------- |
| Chain name                     | `robinhood`                                                          |
| Coordinator deployment name    | `robinhood-24-87-85`                                                 |
| Coordinator salt               | `Coordinator:robinhood:v1.0.13`                                      |
| Wasm contract admin            | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp`                      |
| Governance address             | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`                      |
| Prover operational admin       | `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf`                      |
| Service name                   | `amplifier`                                                          |
| Voting / signing threshold     | `51/100`                                                             |
| Voting block expiry            | `50`                                                                 |
| Configured confirmation height | `1`                                                                  |
| Encoder / key type             | `abi` / `ecdsa`                                                      |
| Source gateway                 | `0x2a949565682ad89ca4Ad325499C36d3865a3ee02`                         |
| Domain separator               | `0x790b531d918bad40a2fac786b46518feaea4fec71b1d7f2a43cce0bb70e0a1fc` |

## Deployment sequence and validation

The [Arc CosmWasm GMP runbook](2025-11-Arc-GMP-v6.0.6.md) describes the shared workflow: predict the EVM gateway, instantiate contracts through Coordinator, save the deployed addresses, register the deployment, create reward pools, register the chain with the router, authorize the prover and configure verifiers. Apply Robinhood's versions and parameters above when following that workflow.

Coordinate ITS Hub registration with the [Robinhood ITS release](../evm/2026-09-Robinhood-ITS-v2.2.0.md). Confirm router registration, prover authorization, verifier participation and bidirectional GMP before considering the connection validated. The contract deployment record alone does not establish completion of those checks.

## Mainnet

Mainnet timing is TBD. Record the mainnet code IDs, addresses, admins, thresholds, governance proposal IDs and validation results in this release before marking mainnet complete.
