# Consensus chains P2P token migration to Axelar ITS hub

|                      | **Owner**                             |
| -------------------- | ------------------------------------- |
| **Created By**       | @kulikthebird <tomasz@interoplabs.io> |


| **Network**          | **Squid Tokens Registration**       | **Date**   |
| -------------------- | ----------------------------------- | ---------- |
| **Testnet**          | TBD                                 | 20xx-xx-xx |
| **Mainnet**          | TBD                                 | 20xx-xx-xx |


| **Network**          | **Other Tokens Rgistration**        | **Date**   |
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



### Migration steps

1. Prepare Mnemonics

The prerequisit step before migration is to store the mnemonic of the **InterchainTokenService operator** account (env: MNEMONIC) in the `.env` file. This should be changed per environment before running scripts.
```bash
MNEMONIC="[ITS operator mnemonic]"
ENV="[environment]"
```

1. Squid tokens migration

All the Squid-enabled tokens can be found in the squid config files. The purpose of this step is to read the configs to find the p2p tokens and run the migration scripts on them.

Before runninge the scripts, copy and paste the squid token config files (two `squid.tokenlist.json` files) into:

```bash
 % axelar-chains-config/info/tokens-p2p/tokens-testnet.json
 % axelar-chains-config/info/tokens-p2p/tokens-mainnet.json
```

Then run the following script in order to register the tokens on testnet ITS Hub (for `testnet` and `mainnet` separatelly):

```bash
ts-node cosmwasm/register-p2p-tokens.ts register-tokens
```


1. Check the tokens were migrated properly

Run the command and check the output. Skipped tokens are the one that were successfully registered on the ITS hub. Tokens that are not skipped are the one that should be migrated once again.

```bash
ts-node cosmwasm/register-p2p-tokens.ts register-tokens --dryRun
```

1. Fetch tokens that are not listed in Squid configs

One can find a pre-fetched tokens stored on the branch [chore(its,evm): p2p tokens found](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1169). These should be placed in:

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


1. Check the tokens were migrated properly

Run the command and check the output. Skipped tokens are the one that were successfully registered on the ITS hub. Tokens that are not skipped are the one that should be migrated once again.

```bash
ts-node cosmwasm/register-p2p-tokens.ts register-tokens --dryRun
```

1. Set trusted chains to `Axelar` on each consensus chain.

**TODO**

Before proceeding set the following variables in the `.env` file:
```bash
PRIVATE_KEY_STELLAR=""
PRIVATE_KEY_SUI=""
PRIVATE_KEY_EVM=""
```

```bash
ts-node common/its.ts set-trusted-chains-all
```


1. Align token supply per chain

The command `align-supply` from the `register-p2p-tokens.ts` script should be run to align the tokens supply per chain. This step is needed, since between the token registration and the ITS contract migration some of the tokens potentially could be transfered between the chains. The command will update the values to be as close to the real ones as possible.
