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


| **Network**          | **ITS v2.2.0 contracts migration**  | **Date**   |
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


## Description of the problem

Before ITS hub the tokens were deployed to work in a peer-2-peer manner. This standard was only used for the EVM-compatible chains. The token contracts are communicating via GMP protocol with each other i.e. for a given token that is deployed on chains X, Y and Z, the X's token contract sends messages directly to the Y's and Z's token contracts of a given token.

From the technical point of view it means that the list of the trusted chains of a given token cotract was set to all the chains that token is connected to. The purpose of this migration is to redirect the communication channel to the ITS hub contract that is deployed on the Axelar network. The goal of this migration procedure is to achieve the following state:
1. List of the trusted chains of each ITS contract need to be set to Axelar chain only.
2. The token contracts need to be registered in the ITS hub by using the message [RegisterP2pTokenInstance](https://github.com/axelarnetwork/axelar-amplifier/blob/b58d789c2b91d245d3593b445e00e9ab8e878ac4/contracts/interchain-token-service/src/msg.rs#L65) # skip-check
3. After the above two steps, the ITS contract on the consensus chains need to be upgraded to [`v2.2.0`](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0) to enforce routing the interchain transfers through the ITS hub on the Axelar network.
4. Last step is to run the per chain token supply align command to make sure that the actual tokens' supply matches the one registered on the ITS hub.


## Scripts needed

The following list of scripts are needed to achieve the goal:
 - `cosmwasm/get-p2p-tokens.ts` - **Not implemented** Fetches RPCs of the chains that have deployed p2p tokens to find them and store in a single config file.
 - `cosmwasm/register-p2p-tokens.ts` - Introduces three commands:
   * `register-tokens` - Calls [RegisterP2pTokenInstance](https://github.com/axelarnetwork/axelar-amplifier/blob/b58d789c2b91d245d3593b445e00e9ab8e878ac4/contracts/interchain-token-service/src/msg.rs#L65) for each legacy p2p token deployed on a consensus chain to register it in the central hub. # skip-check
   * `check-tokens` - Queries the ITS contract on the Axelar network to fetch information about the registration status of a given token per chain.
   * `align-supply` - **Not implemented** This command queries all the tokens to get the current supply per chain and updates the supply value stored on the ITS hub.

The `register-p2p-tokens` script is prepared to work for both squid tokens config file and the output of the `get-p2p-tokens` script.


## Scripts testing

A new interchain token was deployed on the network using:
```bash
ts-node evm/interchainTokenFactory.js deploy-interchain-token --name "test" --symbol "TST" --decimals 18 --initialSupply 12345 --minter [wallet] --salt "salt" -n core-ethereum -e devnet-amplifier
```

After the deployment, the script `cosmwasm/get-p2p-tokens.ts` was run to fetch the latest tokens from `core-ethereum` - the new token has been found. The token was stored in the auto-generated file `axelar-contract-deployments/axelar-chains-config/info/tokens-p2p/tokens-devnet-amplifier.json`. To isolate the token for the test purpose all the other tokens were removed from the file to avoid any unwanted registrations. The `track` flag needed to be set to `true` so that the token is taken into account by the registration script. Then the following command was run:

```bash
ts-node cosmwasm/register-p2p-tokens.ts check-tokens -e devnet-amplifier
```

The new token was checked for the registration status:

```bash
Token 0x156d635b12b653c8dfb6966c1336b722fd9bd8ec01243a33fb1b864eebc5fab8 on core-ethereum is not registered
```

The last step was to run the registration command:

```bash
ts-node cosmwasm/register-p2p-tokens.ts register-tokens -e devnet-amplifier
```

Result:

```bash
Registering token : {"chain":"core-ethereum","token_id":"156d635b12b653c8dfb6966c1336b722fd9bd8ec01243a33fb1b864eebc5fab8","origin_chain":"core-ethereum","decimals":18,"supply":{"tracked":"12345000000000000000000"}}

Token 0x156d635b12b653c8dfb6966c1336b722fd9bd8ec01243a33fb1b864eebc5fab8 on core-ethereum is registered
```


## Squid tokens migration

All the Squid-enabled tokens can be found in the squid config files. The purpose of this step is to read the configs to find the legacy tokens and run the migration scripts on them.


## Testnet & non-squid tokens migration

All the other legacy tokens should be fetched using `get-p2p-tokens` script and then processed by the `register-p2p-tokens`. It is expected to encounter some unusual situations when fetching the tokens from testnet (conflicting ITS token addresses, stored decimals different between chains, tokens stored multiple times etc.). The result of the token fetching script should be analyzed and a proper steps should be implemented in the registration program to handle them.


## Check the tokens were migrated properly

The `register-p2p-tokens` script can be used to check whether the tokens were registered successfully using command `check-tokens`


## Migrate the ITS contracts on the consensus chains

The new [`ITS v2.2.0`](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0) solidity contract changes the intechain token transfers routing from P2P manner to ITS hub only. This PR [feat: make all calls go through the hub](https://github.com/axelarnetwork/interchain-token-service/pull/332) makes the redirection to the hub mandatory (no P2P GMP based communication support). This step should be processed after all the legacy tokens are registered on the ITS Hub.


## Align token supply per chain

The command `align-supply` from the `register-p2p-tokens.ts` script should be run to align the tokens supply per chain. This step is needed, since between the token registration and the ITS contract migration some of the tokens potentially could be transfered between the chains. The command will update the values to be as close to the real ones as possible.
