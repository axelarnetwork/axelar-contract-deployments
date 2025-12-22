# Consensus chains P2P token migration to Axelar ITS hub

|                      | **Owner**                             |
| -------------------- | ------------------------------------- |
| **Created By**       | @kulikthebird <tomasz@interoplabs.io> |
| **Deployment**       | @kulikthebird <tomasz@interoplabs.io> |


| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Testnet**          | Completed             | 2025-12-13 |
| **Mainnet**          | Completed             | 2025-12-17 |


### Background

Without the [Axelar ITS Hub](https://github.com/axelarnetwork/axelar-amplifier/tree/main/contracts/interchain-token-service) solution, the ITS contracts deployed on EVM-compatible chains communicate with each other in a peer-to-peer manner using the Axelar GMP protocol. This migration makes it mandatory for edge ITS contracts to use the ITS hub stored on the Axelar network in order to send interchain transfers.


### Deployment & checks

1. Prepare Mnemonics

The prerequisite step before migration is to store the mnemonic of the **InterchainTokenService operator** account in the `.env` file. This should be changed per environment before running scripts.
```bash
MNEMONIC="[ITS operator mnemonic]"
ENV="[mainnet | testnet | stagenet | devnet-amplifier]"
```

1. Squid tokens migration

All the Squid-enabled tokens can be found in the squid config files. The purpose of this step is to read the configs to find the p2p tokens and run the migration scripts on them.

Before running the scripts, copy and paste the squid token config files (two `squid.tokenlist.json` files) into:

```bash
 % axelar-chains-config/info/tokens-p2p/tokens-testnet.json
 % axelar-chains-config/info/tokens-p2p/tokens-mainnet.json
```

1. Run the following script in order to register the tokens on testnet ITS Hub (for `testnet` and `mainnet` separately):

Scripts can be found in this [PR #1136](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1136) if they are not present on the main branch.

```bash
ts-node cosmwasm/migrate/register-p2p-tokens.ts register-tokens
```


### Checklist

Run the command and check the output. Skipped tokens are the ones that were successfully registered on the ITS hub. Tokens that are not skipped are the ones that should be migrated once again.

```bash
ts-node cosmwasm/migrate/register-p2p-tokens.ts register-tokens --dryRun
```
