# Robinhood ITS v2.2.0

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

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0)

## Background

Deployment was completed with `axe`.

Completed ITS v2.2.0 deployment on Robinhood Chain Testnet (`robinhood`). Addresses and parameters are recorded in [testnet.json](../../axelar-chains-config/info/testnet.json).

## Contracts

| Contract               | Version | Address                                      |
| ---------------------- | ------- | -------------------------------------------- |
| InterchainTokenService | `2.2.0` | `0x3270540e5d2857E69BEC34c6EcFa02cD813aCb42` |
| InterchainTokenFactory | `2.2.0` | `0xDe94329DB2D8C283E2fe6D4e090508D82cd06097` |

Both contracts are owned by `0x49845e5d9985d8dc941462293ed38EEfF18B0eAE`, which is also the ITS operator. The deployer `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` no longer has the ITS operator role.

## Trusted chains

Verified reciprocal trust with Robinhood on September 29, 2026:

solana, flow, ethereum-sepolia, Avalanche, polygon-sepolia, base-sepolia, arbitrum-sepolia, optimism-sepolia.

## Validation

- Flow to Robinhood and Robinhood to Flow ITS transfers completed with manual relay assistance.
- [Solana to Robinhood](https://testnet.axelarscan.io/gmp/4bs4KrqTuGrMtrdATVAp4pvDzvzQWzYMfgCtCuTRXyHTxufWJMrD91TD42mzm2EfAzaJjAzZumPoNnoMPig9s8hD-1.7) delivered 0.01 test AXE through the automatic relayer in approximately 36 seconds. The receiver balance was verified on Robinhood.

Related releases: [EVM GMP](2026-09-Robinhood-GMP.md), [CosmWasm GMP](../cosmwasm/2026-09-Robinhood-GMP.md).
