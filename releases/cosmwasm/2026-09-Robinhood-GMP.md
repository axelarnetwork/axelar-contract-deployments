# Robinhood CosmWasm GMP

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

- [Amplifier Releases](https://github.com/axelarnetwork/axelar-amplifier/releases)
- [VotingVerifier v2.0.2](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v2.0.2)
- [Gateway v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/gateway-v1.1.1)
- [MultisigProver v1.2.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-prover-v1.2.1)

## Background

Deployment was completed with `axe`.

Completed deployment of the Amplifier contracts for the `robinhood` testnet connection. Addresses and parameters are recorded in [testnet.json](../../axelar-chains-config/info/testnet.json).

## Contracts

| Contract       | Version | Code ID | Address                                                             |
| -------------- | ------- | ------- | ------------------------------------------------------------------- |
| VotingVerifier | `2.0.2` | `87`    | `axelar1995v0ysau2cqqedjw2zh5luqlfpnn49hrhtdyhp9lxf72c765upsgupqld` |
| Gateway        | `1.1.1` | `24`    | `axelar1k4jsc5tf4tgdqupzj3a89a9glx63s9vuzrypfsc9qgmkmlymw8hss80c93` |
| MultisigProver | `1.2.1` | `85`    | `axelar1r5yr5u8c6gdy75k3kffletp3clz5cg96e8h0sv8t5gew83f43z8q8dpvzz` |

## Registration

The Coordinator deployment is `robinhood-24-87-85`. The Router registration is unfrozen, the MultisigProver is authorized on Multisig, and the Robinhood ITS contract is registered and unfrozen on the ITS Hub.

The upgrade admin for all three CosmWasm contracts is `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp`.

Related releases: [EVM GMP](../evm/2026-09-Robinhood-GMP.md), [ITS v2.2.0](../evm/2026-09-Robinhood-ITS-v2.2.0.md).
