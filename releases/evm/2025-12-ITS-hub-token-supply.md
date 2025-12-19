# Consensus chains P2P token migration to Axelar ITS hub

|                      | **Owner**                             |
| -------------------- | ------------------------------------- |
| **Created By**       | @kulikthebird <tomasz@interoplabs.io> |
| **Deployment**       | @kulikthebird <tomasz@interoplabs.io> |


| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet amplifier** | Completed             | 2025-12-16 |
| **Stagenet**         | Completed             | 2025-12-16 |
| **Testnet**          | Completed             | 2025-12-16 |
| **Mainnet**          | Completed             | 2025-12-18 |


### Background

Without the [Axelar ITS Hub](https://github.com/axelarnetwork/axelar-amplifier/tree/main/contracts/interchain-token-service) solution, the ITS contracts deployed on EVM-compatible chains communicate with each other in a peer-to-peer manner using the Axelar GMP protocol. This migration makes it mandatory for edge ITS contracts to use the ITS hub stored on the Axelar network in order to send interchain transfers.


### Deployment

1. Prepare Mnemonics

The prerequisite step before migration is to store the mnemonic of the **InterchainTokenService operator** account in the `.env` file. This should be changed per environment before running scripts.
```bash
MNEMONIC="[ITS operator mnemonic]"
ENV="[mainnet | testnet | stagenet | devnet-amplifier]"
```


1. Set `axelar` as a trusted chain on each EVM chain that has ITS deployed.

```bash
ts-node evm/its.js set-trusted-chains all -n [consensus chains]
```


1. Align token supply registered on ITS hub.

Scripts can be found in this [PR #1136](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1136) if they are not present on the main branch.

Run the following command to align token supply per each environment:

```bash
ts-node cosmwasm/migrate/register-p2p-tokens.ts align-token-supply
```

**Note:** This command should be run for both squid & fetched tokens. The config file should be placed in `axelar-chains-config/info/tokens-p2p/tokens-${env}.json`.

Pre-fetched (non-squid) tokens can be found in this [PR #1255](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1255).

Squid tokens can be found in the config repository.

### Checklist

Perform an ITS token transfer for several migrated tokens to make sure the tokens were migrated successfully.
