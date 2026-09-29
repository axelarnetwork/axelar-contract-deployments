# Robinhood GMP

| Network | Deployment status                                                                                                                | Date           |
| ------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| Testnet | Contracts deployed; configuration recorded in [PR #1418](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1418) | September 2026 |
| Mainnet | TBD                                                                                                                              | TBD            |

## Background

This release records the Robinhood Chain Testnet EVM deployment. The chain uses Axelar's Amplifier connection. The deployed addresses and parameters are recorded under `chains.robinhood` in [testnet.json](../../axelar-chains-config/info/testnet.json).

Related releases:

- [Robinhood CosmWasm GMP](../cosmwasm/2026-09-Robinhood-GMP.md)
- [Robinhood ITS v2.2.0](2026-09-Robinhood-ITS-v2.2.0.md)

## Testnet deployment

| Parameter                                         | Value                                          |
| ------------------------------------------------- | ---------------------------------------------- |
| Axelar chain name                                 | `robinhood`                                    |
| EVM chain ID                                      | `46630`                                        |
| RPC                                               | `https://rpc.testnet.chain.robinhood.com`      |
| Explorer                                          | `https://explorer.testnet.chain.robinhood.com` |
| Native token                                      | ETH, 18 decimals                               |
| Finality                                          | `finalized`                                    |
| Gateway deployment method                         | `create`                                       |
| Previous signer sets retained                     | `15`                                           |
| Minimum rotation delay                            | `3600` seconds                                 |
| Deployer / gateway operator                       | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC`   |
| Recorded gateway, Operators and gas service owner | `0x49845e5d9985d8dc941462293ed38EEfF18B0eAE`   |

| Contract                     | Address                                      |
| ---------------------------- | -------------------------------------------- |
| ConstAddressDeployer         | `0x39fD42c43D27464b64B120E981CfEe68351D9F49` |
| Create3Deployer              | `0x4A0fc57047cc99Cb28152730A42F5a432Aece667` |
| AxelarGateway proxy          | `0x2a949565682ad89ca4Ad325499C36d3865a3ee02` |
| AxelarGateway implementation | `0x8E60A250B6E5778B2342a3Fa1d11C145E8128d1f` |
| Operators                    | `0xaC6E171c9A8eF95c943F512e54712B8006c33E3C` |
| AxelarGasService proxy       | `0xef2e20AE1095C6bDb888e91E3c1a2443CBb76DB9` |

The gateway proxy was deployed on September 18, 2026 in [transaction 0x0b7e…b7d0](https://explorer.testnet.chain.robinhood.com/tx/0x0b7e154fb3ea660cf879953700b96731bcae95a8fd8c0860ac951ef5f2adb7d0). Its `proxyDeploymentArgs` are the original constructor arguments recovered from that transaction: implementation, initial owner, and encoded operator/signer setup. The initial owner was the deployer, so it differs from the recorded current owner.

## Deployment sequence and validation

Use the [Arc EVM GMP runbook](2025-11-Arc-GMP-v6.0.6.md#steps) as the workflow reference: deploy the address deployers, coordinate the predicted EVM gateway address with the CosmWasm deployment, deploy the gateway, deploy Operators and the gas service, configure operators, and transfer ownership. Use Robinhood's recorded parameters above; Arc's addresses, salts and package versions are specific to its release.

For deployment verification, follow [EVM contract verification](../../evm/README.md#contract-verification). Validate GMP in both directions using the source/destination checks in the Arc runbook with Robinhood's chain name and addresses. This deployment record does not attest to completion of end-to-end GMP tests.

## Mainnet

Mainnet timing and deployment parameters are TBD. Before a mainnet deployment, update this release and the linked CosmWasm/ITS releases with the approved chain configuration, contract versions, deployers, owners, verifier thresholds, governance proposals and validation results.
