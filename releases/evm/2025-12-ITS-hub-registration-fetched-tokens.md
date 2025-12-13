# Consensus chains P2P token migration to Axelar ITS hub

|                      | **Owner**                             |
| -------------------- | ------------------------------------- |
| **Created By**       | @kulikthebird <tomasz@interoplabs.io> |
| **Deployment**       | @kulikthebird <tomasz@interoplabs.io> |


| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet amplifier** | TBD                   | 20xx-xx-xx |
| **Stagenet**         | TBD                   | 20xx-xx-xx |
| **Testnet**          | TBD                   | 20xx-xx-xx |
| **Mainnet**          | TBD                   | 20xx-xx-xx |


### Background

Without the [Axelar ITS Hub](https://github.com/axelarnetwork/axelar-amplifier/tree/main/contracts/interchain-token-service) solution, the ITS contracts deployed on EVM-compatible chains communicate with each other in a peer-to-peer manner using the Axelar GMP protocol. This migration makes it mandatory for edge ITS contracts to use the ITS hub stored on the Axelar network in order to send interchain transfers.


### Deployment

1. Prepare Mnemonics

The prerequisite step before migration is to store the mnemonic of the **InterchainTokenService operator** account in the `.env` file. This should be changed per environment before running scripts.
```bash
MNEMONIC="[ITS operator mnemonic]"
ENV="[mainnet | testnet | stagenet | devnet-amplifier]"
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


1. Register fetched legacy tokens

```bash
ts-node cosmwasm/migrate/register-p2p-tokens.ts register-tokens
```

### Checklist

Run the command and check the output. Skipped tokens are the ones that were successfully registered on the ITS hub. Tokens that are not skipped are the ones that should be migrated once again.

```bash
ts-node cosmwasm/migrate/register-p2p-tokens.ts register-tokens --dryRun
```
