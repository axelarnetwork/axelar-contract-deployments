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


1. Set `axelar` as a trusted chain on each EVM chain that has ITS deployed.

```bash
ts-node evm/its.js set-trusted-chains axelar -n all
```


1. Align token supply registered on ITS hub.

Run the following command to align token supply per each environment:

```bash
ts-node cosmwasm/migrate/register-p2p-tokens.ts align-token-supply
```

**Note:** This command should be run for both squid & non-squid tokens

### Checklist

Perform an ITS token transfer for several migrated tokens to make sure the tokens were migrated successfully.
