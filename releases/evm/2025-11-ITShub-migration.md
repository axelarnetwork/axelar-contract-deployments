# ITS p2p token migration to ITS hub 2.1

## Description of the problem

Before ITS hub the tokens were deployed to work in a peer-2-peer manner. This standard was only used for the EVM-compatible chains. The token contracts are communicating via GMP protocol with each other i.e. for a given token that is deployed on chains X, Y and Z, the X's token contract sends messages directly to the Y's and Z's token contracts of a given token.

From the technical point of view it means that the list of the trusted chains of a given token cotract was set to all the chains that token is connected to. The purpose of this migration is to redirect the communication channel to the ITS hub contract that is deployed on the Axelar network. The goal of this migration procedure is to achieve the following state:
1. List of the trusted chains of each ITS contract need to be set to Axelar chain only.
2. The token contracts need to be registered in the ITS hub by using the message [RegisterP2pTokenInstance](https://github.com/axelarnetwork/axelar-amplifier/blob/b58d789c2b91d245d3593b445e00e9ab8e878ac4/contracts/interchain-token-service/src/msg.rs#L65) # skip-check


## Finished steps

[ ] Finish the scripts implementation/refactoring
[ ] Prepare test scenarios for the scripts
[ ] Prepare post-migration checks to ensure the valid state of the migrated tokens
[ ] Migrate Squid tokens
[ ] Migrate Testnet / non-Squid Mainnet tokens


## Scripts needed

The following list of scripts are needed to achieve the goal:
 - `get-token-manager-deployments` - Fetches RPCs of the chains that have deployed p2p tokens to find them and store in a single config file.
 - `get-token-info` - Fetches token info for the found tokens that is needed for the migration.
 - `register-its-token` - Calls [RegisterP2pTokenInstance](https://github.com/axelarnetwork/axelar-amplifier/blob/b58d789c2b91d245d3593b445e00e9ab8e878ac4/contracts/interchain-token-service/src/msg.rs#L65) for each p2p token to be registered in the central hub. # skip-check


## Scripts testing

### Scenario 1
### Scenario 2


## Squid tokens migration

All the Squid-enabled tokens can be found in the squid config files. The purpose of this step is to read the configs to find the legacy tokens and run the migration scripts on them.


## Testnet & non-squid tokens migration

All the other legacy tokens should be fetched and run through the same migration procedure.



## Check the tokens were migrated properly

**TODO** prepare the checklist
