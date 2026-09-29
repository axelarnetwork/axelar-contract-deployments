# Robinhood GMP

|                | **Owner**                               |
| -------------- | --------------------------------------- |
| **Created By** | @makischristou <makis@commonprefix.com> |
| **Deployment** | @makischristou <makis@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**       |
| -------------------- | --------------------- | -------------- |
| **Devnet Amplifier** | -                     | TBD            |
| **Stagenet**         | -                     | TBD            |
| **Testnet**          | Complete              | September 2026 |
| **Mainnet**          | -                     | TBD            |

- [Releases](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/releases)

## Background

Deployment was completed with `axe`.

Completed deployment of the Amplifier EVM contracts on Robinhood Chain Testnet (`robinhood`, chain ID `46630`). The deployed addresses and parameters are recorded in [testnet.json](../../axelar-chains-config/info/testnet.json).

## Contracts

| Contract             | Address                                      |
| -------------------- | -------------------------------------------- |
| ConstAddressDeployer | `0x39fD42c43D27464b64B120E981CfEe68351D9F49` |
| Create3Deployer      | `0x4A0fc57047cc99Cb28152730A42F5a432Aece667` |
| AxelarGateway        | `0x2a949565682ad89ca4Ad325499C36d3865a3ee02` |
| Operators            | `0xaC6E171c9A8eF95c943F512e54712B8006c33E3C` |
| AxelarGasService     | `0xef2e20AE1095C6bDb888e91E3c1a2443CBb76DB9` |

## Ownership and operation

AxelarGateway, Operators, and AxelarGasService are owned by `0x49845e5d9985d8dc941462293ed38EEfF18B0eAE`. The gateway operator is `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC`. The gas collector is the Operators contract.

Source-message verification waits for the RPC's `finalized` block tag.

Related releases: [CosmWasm GMP](../cosmwasm/2026-09-Robinhood-GMP.md), [ITS v2.2.0](2026-09-Robinhood-ITS-v2.2.0.md).
