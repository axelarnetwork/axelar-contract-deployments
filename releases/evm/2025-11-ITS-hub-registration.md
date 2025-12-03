# Consensus chains P2P token migration to Axelar ITS hub

|                      | **Owner**                             |
| -------------------- | ------------------------------------- |
| **Created By**       | @kulikthebird <tomasz@interoplabs.io> |


| **Network**          | **Squid Tokens Registration**       | **Date**   |
| -------------------- | ----------------------------------- | ---------- |
| **Testnet**          | TBD                                 | 20xx-xx-xx |
| **Mainnet**          | TBD                                 | 20xx-xx-xx |


| **Network**          | **Other Tokens Registration**        | **Date**   |
| -------------------- | ----------------------------------- | ---------- |
| **Devnet amplifier** | TBD                                 | 20xx-xx-xx |
| **Stagenet**         | TBD                                 | 20xx-xx-xx |
| **Testnet**          | TBD                                 | 20xx-xx-xx |
| **Mainnet**          | TBD                                 | 20xx-xx-xx |


| **Network**          | **Tokens supply alignment**         | **Date**   |
| -------------------- | ----------------------------------- | ---------- |
| **Devnet amplifier** | TBD                                 | 20xx-xx-xx |
| **Stagenet**         | TBD                                 | 20xx-xx-xx |
| **Testnet**          | TBD                                 | 20xx-xx-xx |
| **Mainnet**          | TBD                                 | 20xx-xx-xx |


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

Then run the following script in order to register the tokens on testnet ITS Hub (for `testnet` and `mainnet` separately):

```bash
ts-node cosmwasm/register-p2p-tokens.ts register-tokens
```


1. Check that the tokens were migrated properly

Run the command and check the output. Skipped tokens are the ones that were successfully registered on the ITS hub. Tokens that are not skipped are the ones that should be migrated once again.

```bash
ts-node cosmwasm/register-p2p-tokens.ts register-tokens --dryRun
```

1. Fetch tokens that are not listed in Squid configs

One can find pre-fetched tokens stored on the branch [chore(its,evm): p2p tokens found](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1169). These should be placed in:

```bash
 % axelar-chains-config/info/tokens-p2p/tokens-devnet-amplifier.json
 % axelar-chains-config/info/tokens-p2p/tokens-stagenet.json
 % axelar-chains-config/info/tokens-p2p/tokens-testnet.json
 % axelar-chains-config/info/tokens-p2p/tokens-mainnet.json
```

One should run the following script for each environment to make sure the configs are up-to-date before the migration:

```bash
 % cosmwasm/migrate/get-p2p-tokens.ts -e devnet-amplifier
 % cosmwasm/migrate/get-p2p-tokens.ts -e stagenet
 % cosmwasm/migrate/get-p2p-tokens.ts -e testnet
 % cosmwasm/migrate/get-p2p-tokens.ts -e mainnet
```

**Note:** The scripts can be run in parallel.


1. Check that the tokens were migrated properly

Run the command and check the output. Skipped tokens are the ones that were successfully registered on the ITS hub. Tokens that are not skipped are the ones that should be migrated once again.

```bash
ts-node cosmwasm/register-p2p-tokens.ts register-tokens --dryRun
```

1. Set `axelar` as a trusted chain on each EVM chain that has ITS deployed.

```bash
ts-node evm/its.js set-trusted-chains axelar -n all
```


1. Align token supply registered on ITS hub.

Run the following command to align token supply per each environment:

```bash
ts-node cosmwasm/register-p2p-tokens.ts modify-token-supply
```
