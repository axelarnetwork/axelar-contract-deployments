# Sui Deployment Guide

## Table of Contents

-   [Prerequisites](#prerequisites)
-   [Deployment](#deployment)
-   [Contract Upgrades](#contract-upgrades)
-   [Contract Interactions](#contract-interactions)
-   [Examples](#examples)
-   [Troubleshooting](#troubleshooting)

## Prerequisites

Install Sui CLI: `brew install sui`

A Sui keypair can be created as follows.

1. Using Sui CLI:

```bash
sui client new-address secp256k1 wallet

# Export private key in bech32 format
sui keytool export --key-identity wallet
```

2. Using the script

```bash
ts-node sui/generate-keypair.js
```

Set `PRIVATE_KEY="suiprivkey..."` in your `.env` file. Other private key types are supported via `--privateKeyType` and `--signatureScheme` flags.

If you want to run against a local Sui network, then create a `axelar-chains-config/info/local.json` config containing:

```json
{
    "chains": {
        "sui": {
            "name": "Sui",
            "axelarId": "sui",
            "networkType": "localnet",
            "chainType": "sui",
            "tokenSymbol": "SUI",
            "rpc": "http://127.0.0.1:9000",
            "faucetUrl": "http://127.0.0.1:9123",
            "contracts": {
                "AxelarGateway": {}
            }
        }
    }
}
```

Use the `-e local` (or `ENV=local` in the `.env` config) flag with scripts to run against the local network.

## Deployment

To get test SUI coins to your address via a faucet.

```bash
ts-node sui/faucet.js
```

The following packages need to be deployed in order because they are referenced by other packages.

Command syntax:

```bash
ts-node sui/deploy-contract.js deploy <package name> [--policy <policy>]
```

Where the policy can be one of the following:

- `any_upgrade` (default): Allow any upgrade.
- `code_upgrade`: Upgrade policy to just add code.
- `dep_upgrade`: Upgrade policy to just change dependencies.
- `immutable`: Make the package immutable.

#### Utils

```bash
ts-node sui/deploy-contract.js deploy Utils
```

### Version Control

```bash
ts-node sui/deploy-contract.js deploy VersionControl
```

### AxelarGateway

-   By querying the signer set from the Amplifier contract (this only works if Amplifier contracts have been setup):

```bash
ts-node sui/deploy-contract.js deploy AxelarGateway
```

Note: the `minimumRotationDelay` is in `seconds` unit. The default value is `24 * 60 * 60` (1 day).

Use `--help` flag to see other setup params that can be overridden.

-   For testing convenience, you can use the secp256k1 wallet as the signer set for the gateway.

```bash
ts-node sui/deploy-contract.js deploy AxelarGateway --signers wallet --nonce test
```

-   You can also provide a JSON object with a full signer set:

```bash
ts-node sui/deploy-contract.js deploy AxelarGateway -e testnet --signers '{"signers": [{"pub_key": "0x020194ead85b350d90472117e6122cf1764d93bf17d6de4b51b03d19afc4d6302b", "weight": 1}], "threshold": 1, "nonce": "0x0000000000000000000000000000000000000000000000000000000000000000"}'
```

### Gas Service

```bash
ts-node sui/deploy-contract.js deploy GasService
```

### Abi

```bash
ts-node sui/deploy-contract.js deploy Abi
```

### Operators

```bash
ts-node sui/deploy-contract.js deploy Operators
```

#### Operators Post Deployment

##### Gas Collector Configuration

To allow the operator to collect or refund gas, the `GasCollector` cap must be stored in the `Operators` contract:

```bash
ts-node sui/operators.js storeCap
```

##### Operator Management

Assign `Operator` role to given address:

```bash
ts-node sui/operators.js add <operator address>
```

### Governance

```bash
ts-node sui/deploy-contract.js deploy Governance
```

### Relayer Discovery

```bash
ts-node sui/deploy-contract.js deploy RelayerDiscovery
```

### ITS

```bash
ts-node sui/deploy-contract.js deploy ITS
```

#### Interchain Transfer

```bash
ts-node sui/its.js interchain-transfer \
   coin-package-id \
   coin-package-name \
   coin-mod-name \
   coin-object-id \
   token-id \
   destination-chain \
   destination-address \
   amount \
   --env <environment> \
   --signatureScheme <your_sig_scheme>
```

```bash
ts-node sui/its.js interchain-transfer \
  0x1e280e60f86e4c5b46d9fc39795d0253430dd3a53acc19edb080c6284de8322b \
  my_custom_coin \
  MY_CUSTOM_COIN \
  0x9a51b93a9ae535e6d5969f3d714d1488f9edb47c1711803982454e403d2f266f \
  0xc6acd961da74eed78fbe714ab7bb31bbf88ea6e2f92fd3f2a8721c6789eb0d4a \
  ethereum-sepolia \
  0xc5DcAC3e02f878FE995BF71b1Ef05153b71da8BE \
  1 \
  --env testnet \
  --signatureScheme ed25519
```

Response

````bash
Interchain Transfer: [4WFznnh4UM81e5fEfBpfCbY1prA5WVmXFBBKmuhb7tQz](https://testnet.axelarscan.io/gmp/4WFznnh4UM81e5fEfBpfCbY1prA5WVmXFBBKmuhb7tQz-3)
````

### Squid

```bash
ts-node sui/deploy-contract.js deploy Squid
```

### Example

```bash
ts-node sui/deploy-contract.js deploy Example
```

## Sync Packages

This command synchronizes local Move packages with their deployed addresses. This is useful when you don't have all published packages locally but need to deploy a package that depends on others.

**Functionality:**

1.  **Copies Move Packages:** Copies all Move packages from `node_modules/@axelar-network/axelar-cgp-sui/move` to the local `sui/move` directory.
2.  **Updates `Move.toml`:** Updates the `Move.toml` file for each package, replacing the package name with the corresponding package ID from the `axelar-chain-configs/info/<env>.json` configuration file.

**Use Case:**

This is useful when:

- You want to deploy or upgrade a specific package.
- This package has dependencies on other packages.
- You do _not_ have all the published packages (and their `Move.toml` files with correct addresses) locally.

**Example:**

To deploy `InterchainTokenService`, which depends on other packages, and you don't have the up-to-date `Move.toml` files for all dependencies:

```bash
ts-node sui/deploy-contract.js sync
```

This command copies all packages and updates their `Move.toml` files with correct package IDs. Then, you can deploy `InterchainTokenService`:

```bash
ts-node sui/deploy-contract.js deploy InterchainTokenService
```

## Contract Upgrades

### Upgrade Procedures

For example, to update the gateway run the following command:

```bash
ts-node sui/deploy-contract.js upgrade AxelarGateway <policy>
```

policy should be one of the following:

-   `any_upgrade`: Allow any upgrade.
-   `code_upgrade`: Upgrade policy to just add code. https://docs.sui.io/references/framework/sui-framework/package#function-only_additive_upgrades
-   `dep_upgrade`: Upgrade policy to just change dependencies. https://docs.sui.io/references/framework/sui-framework/package#function-only_dep_upgrades

Provide `--txFilePath` with `--offline` to generate tx data file for offline signing.

### Migrating Post-Upgrade

After upgrading a package, state migrations (e.g. for [versioned](https://docs.sui.io/references/framework/sui/versioned) packages) can be called using the `migrate` command.


```bash
ts-node sui/deploy-contract.js migrate AxelarGateway
```

### Multisig Operations

To create a Multisig, follow the documentation [here](https://docs.sui.io/guides/developer/cryptography/multisig).

Get test SUI coins to your multisig address via a faucet:

```bash
sui client faucet --address <multisig address>
```

Get public keys for all wallets:

```bash
sui keytool list
```

Get private key of wallet using wallet alias or address:

```bash
sui keytool export --key-identity <alias/wallet address>
```

Get tx data for testing:

```bash
sui client transfer-sui --to <recipient address> --amount 1 --sui-coin-object-id <sui coin object id> --serialize-unsigned-transaction --gas-budget 77047880
```

To get sui coin object id

```bash
sui client gas <multisig address>
```

Sign transaction block for multisig:

```bash
ts-node sui/multisig.js --txBlockPath <path to unsigned tx block> --signatureFilePath <path to store signature> --action sign --offline
```

example txBlock file:

```
{
    "bytes": "AAACACBC5cSnnYJrDEn9nSW1BDzPLLAbUJbYOeJnUgYl/b90..."
}
```

Combine signature files:

```bash
ts-node sui/multisig.js --txBlockPath <path to unsigned tx block> --signatureFilePath <path to store combined signature> --action combine --offline --signatures <paths to files containing signatures>
```

Execute combined signature:

This command will broadcast the signature to the network

```bash
ts-node sui/multisig.js --txBlockPath <path to unsigned tx block> --action execute --combinedSignPath <path to combined signature>
```

use --multisigKey `multisigKey` to override existing multisig info in chains config

example for adding multisig info to chains config:

```
{
    "sui": {
        "name": "Sui",
        "axelarId": "sui",
        "networkType": "testnet",
        "tokenSymbol": "SUI",
        "rpc": "https://fullnode.testnet.sui.io:443",
        "contracts": {},
        "multisig": {
            "threshold": 2,
            "signers": [
                {
                    "publicKey": "AIqrCb324p6Qd4srkqCzn9NJHS7W17tA7r3t7Ur6aYN",
                    "weight": 1,
                    "schemeType": "ed25519"
                },
                .
                .
                .
            ]
        }
    }
}
```

*Note: To sign via ledger replace private-key with 'ledger' keyword in env and update key scheme to ed25519, as it is the only signatureScheme supported by Ledger currently for Sui. 

## Contract Interactions

### Call Contract

```bash
ts-node sui/gateway.js call-contract ethereum 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

### Pay Gas

The syntax is `ts-node sui/gas-service.js payGas --amount <amount> <destinationChain> <destinationAddress> <channelId> <payload>`

```bash
ts-node sui/gas-service.js payGas --amount 0.1 ethereum 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

### Collect Gas

```bash
ts-node sui/gas-service.js collectGas --amount 0.1 --receiver <receiver address>
```

### Approve Messages

If the gateway was deployed using the wallet, you can submit a message approval with it

```bash
ts-node sui/gateway.js approve --proof wallet --currentNonce test ethereum 0x0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-1 0x4F4495243837681061C4743b74B3eEdf548D56A5 0xa84d27bd6c9680e52e93779b8977bbcb73273b88f52a84d8dd8af1c3301341d7 0x47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad
```

### Rotate Gateway Signers

If gateway was deployed with the wallet as the verifier, and you want to rotate to the Amplifier verifiers, do

```bash
ts-node sui/gateway.js rotate --proof wallet --currentNonce test
```

If you want to rotate to the wallet again but with a new nonce, do

```bash
ts-node sui/gateway.js rotate --signers wallet --proof wallet --currentNonce test --newNonce test2
```

Use the same nonce for `--currentNonce` as the `--nonce` when deploying the gateway.

To submit a proof constructed on Amplifier, run the following with the multisig session id,

```bash
ts-node sui/gateway.js submitProof [multisig session id]
```

### Transfer Object

Please note shared objects cannot be transferred via this script.

```bash
ts-node sui/transfer-object.js --objectId <object id to be transferred> --recipient <recipient address>

ts-node sui/transfer-object.js --contractName <Can be checked from config> --objectName <picked from config> --recipient <recipient address>
```

### Coins Management

List of coins in the wallet:

```bash
ts-node sui/tokens.js list
```

Merge the coins:

```bash
ts-node sui/tokens.js merge --coin-type <coin type to merge>
```

If coin type is not provided, it will merge all the coins.

Split the coins:

```bash
ts-node sui/tokens.js split --amount <amount> --coin-type <coin type to split> --transfer <recipient address>
```

Note:

-   If coin type is not provided, it will split all the coins.
-   If transfer address is not provided, it will split the coins in the same wallet. Otherwise, it will transfer the splitted coins to the provided address.

## Setup Trusted Chains

Add trusted chains

```bash
ts-node sui/its.js add-trusted-chains <sourceChain> <sourceChain2> ...
```

or Add all chains that have ITS contract deployed

```bash
ts-node sui/its.js add-trusted-chains all
```

Remove trusted chains

```bash
ts-node sui/its.js remove-trusted-chains <sourceChain> <sourceChain2> ...
```

## Registering Coins

### Register Coin from Info (symbol, name and decimals)

```bash
ts-node sui/its.js register-coin-from-info <symbol> <name> <decimals>
```

### Register Coin from Metadata 

(see: [sui::coin::CoinMetadata](https://docs.sui.io/references/sui-api/sui-graphql/reference/types/objects/coin-metadata))

```bash
ts-node sui/its.js register-coin-from-metadata <symbol> <name> <decimals>
```

### Register Custom Coin

If a `channel` id is present in the `options` array (e.g. `--channel <channel>`) it will be used, otherwise a new `channel` will be created and transferred to the sender. A `salt` for the registration transaction will automatically be created.

```bash
ts-node sui/its.js register-custom-coin <symbol> <name> <decimals>
```

## Migrating Legacy Coin Registrations

### Migrate Coin Metadata

_Added in v1 to fix coins that were not displaying correctly in wallet softwares. Only callable for coins with metadata owned by ITS. Will [publicly freeze](https://docs.sui.io/references/framework/sui/transfer#sui_transfer_public_freeze_object) a coin's metadata, making it a publicly shared object._

```bash
ts-node sui/its.js migrate-coin-metadata <symbol>
```

## Coin Linking

### Give Unlinked Coin

Deploys a coin on Sui, registers it as custom coin and gives its treasury capability to ITS. Treasury capability will be reclaimable if the `--treasuryCapReclaimer` flag is passed to the command options.

```bash
ts-node sui/its give-unlinked-coin [options] <symbol> <name> <decimals>
```

### Remove Unlinked Coin

Removes a coin from ITS and returns its TreasuryCap to the caller. Caller must own the coin's TreasuryCapReclaimer.

```bash
ts-node sui/its remove-unlinked-coin [options] <symbol>
```

### Link Coin

Deploys a source coin and links it with a destination chain coin. If a `channel` id is present in the `options` array (e.g. `--channel <channel>`) it will be used, otherwise a new `channel` will be created and transferred to the sender. A `salt` for the coin registration and linking transactions will automatically be created.

```bash
ts-node sui/its link-coin <symbol> <name> <decimals> <destinationChain> <destinationAddress>
```

## Treasury Management

### Remove Treasury Cap

Transfers the coin's `TreasuryCap` to the coin deployer and reclaims mint/burn permission from ITS.

```bash
ts-node sui/its remove-treasury-cap [options] <symbol>
```

### Restore Treasury Cap

Restore a coin's TreasuryCap to ITS after calling remove-treasury-cap, giving mint/burn permission back to ITS.

```bash
ts-node sui/its restore-treasury-cap [options] <symbol>
```

## Sui Contract Verification

This script generates a `verification` folder inside the `move` directory, which contains ZIP files for each contract to be used for verification. Before zipping, a `deps` subdirectory is added to each contract, and the local dependency paths in the `Move.toml` file are updated to reference the `deps` folder. 

1. Ensure that same `move` folder is present in `sui` directory which was generated during contract deployment phase. `move` folder generated by other methods will result in failure of the script

2. If all contracts need to be verified use `all` instead of `contractName`

Note:

-   Contracts like `AxelarGateway` have dependencies on `Utils` & `VersionControl` contracts. Make sure these contracts are present and in the `move` folder

```bash
ts-node sui/verify-contract.js <all/contractName> 
```

Post-Command Cleanup Steps:

- Navigate to the `move` folder within the `sui` directory.
- Carefully delete the sub-folder named `verification`, ensuring no other folders are modified.

## Examples

-   [GMP Example Guide](docs/gmp.md)
-   [ITS Example Guide](docs/its.md)

## Troubleshooting

1. Move build error during the deployment step

Delete the `node_modules` folder and `package-lock.json` file and then run `npm install` again.
