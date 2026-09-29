# Robinhood ITS v2.2.0

| Network | Deployment status                                                                                                                | Date           |
| ------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| Testnet | Contracts deployed; configuration recorded in [PR #1418](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1418) | September 2026 |
| Mainnet | TBD                                                                                                                              | TBD            |

[ITS v2.2.0 release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0)

## Background

This release records InterchainTokenService and InterchainTokenFactory v2.2.0 on Robinhood Chain Testnet. It depends on [Robinhood EVM GMP](2026-09-Robinhood-GMP.md) and [Robinhood CosmWasm GMP](../cosmwasm/2026-09-Robinhood-GMP.md).

## Testnet deployment

The deployed addresses are recorded in [testnet.json](../../axelar-chains-config/info/testnet.json) under `chains.robinhood.contracts`.

| Contract               | Version | Proxy                                        | Implementation                               |
| ---------------------- | ------- | -------------------------------------------- | -------------------------------------------- |
| InterchainTokenService | `2.2.0` | `0x3270540e5d2857E69BEC34c6EcFa02cD813aCb42` | `0x4D2D80406E62279bfA7C25e77533c4AAea8f1259` |
| InterchainTokenFactory | `2.2.0` | `0xDe94329DB2D8C283E2fe6D4e090508D82cd06097` | `0x384ec8f93a633f14f210fE3290E6BEF24a2804e9` |

| Parameter                     | Value                                        |
| ----------------------------- | -------------------------------------------- |
| Chain name                    | `robinhood`                                  |
| Deployer / recorded ITS owner | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |
| ITS implementation salt       | `ITS v2.2.0`                                 |
| ITS proxy salt                | `ITS v1.0.0`                                 |
| Factory salt                  | `ITS Factory v1.0.0`                         |

## Deployment sequence and validation

Follow the [Arc ITS v2.2.0 runbook](2025-11-Arc-ITS-v2.2.0.md) for the shared deployment, contract verification, ITS Hub registration and trusted-chain configuration workflow, substituting `robinhood` and the addresses above. The salt values in the table are the values persisted in config; use the CLI salt conventions documented in that runbook.

Register the Robinhood ITS address with ITS Hub in coordination with the CosmWasm release, and configure the required remote ITS contracts to trust Robinhood through the hub. Validate token deployment to a remote chain and transfers in both directions using the runbook's checklist. This document records deployed contracts; it does not claim that Hub registration, trusted-chain configuration or transfer checks have been independently verified.

## Mainnet

Mainnet timing is TBD. Before deployment, record the mainnet addresses, contract versions, ownership, Hub registration proposal and trusted-chain changes here. Record bidirectional token-transfer results before marking the release complete.
