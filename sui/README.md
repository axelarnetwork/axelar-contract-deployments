# Sui deployment scripts

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
node sui/generate-keypair.js
```

Set `PRIVATE_KEY="suiprivkey..."` in your `.env` file. Other private key types are supported via `--privateKeyType` and `--signatureScheme` flags.

If you want to run against a local Sui network, then create a `axelar-chains-config/info/local.json` config containing:

```bash
{
    "sui": {
        "name": "Sui",
        "axelarId": "sui",
        "networkType": "localnet",
        "tokenSymbol": "SUI",
        "rpc": "http://127.0.0.1:9000",
        "faucetUrl": "http://127.0.0.1:9123",
        "contracts": {
          "AxelarGateway": {}
        }
    }
}
```

Use the `-e local` (or `ENV=local` in the `.env` config) flag with scripts to run against the local network.

## Scripts

To get test SUI coins to your address via a faucet.

```bash
node sui/faucet.js
```

Deploy the gateway package:

-   By querying the signer set from the Amplifier contract (this only works if Amplifier contracts have been setup):

```bash
node sui/deploy-contract.js deploy AxelarGateway
```

Note: the `minimumRotationDelay` is in `seconds` unit. The default value is `24 * 60 * 60` (1 day).

Use `--help` flag to see other setup params that can be overridden.

-   For testing convenience, you can use the secp256k1 wallet as the signer set for the gateway.

```bash
node sui/deploy-contract.js deploy AxelarGateway --signers wallet --nonce test
```

-   You can also provide a JSON object with a full signer set:

```bash
node sui/deploy-contract.js deploy AxelarGateway -e testnet --signers '{"signers": [{"pub_key": "0x020194ead85b350d90472117e6122cf1764d93bf17d6de4b51b03d19afc4d6302b", "weight": 1}], "threshold": 1, "nonce": "0x0000000000000000000000000000000000000000000000000000000000000000"}'
```

Upgrading Gateway:

To update the gateway run the following command:

```bash
node sui/deploy-contract.js upgrade AxelarGateway <policy>
```

policy should be one of the following:

-   `any_upgrade`: Allow any upgrade.
-   `code_upgrade`: Upgrade policy to just add code. https://docs.sui.io/references/framework/sui-framework/package#function-only_additive_upgrades
-   `dep_upgrade`: Upgrade policy to just change dependencies. https://docs.sui.io/references/framework/sui-framework/package#function-only_dep_upgrades

Provide `--txFilePath` with `--offline` to generate tx data file for offline signing.

Deploy the Gas Service package:

```bash
node sui/deploy-contract.js deploy GasService
```

Deploy the test GMP package:

```bash
node sui/deploy-contract.js deploy Test
```

Deploy the Operators package:

```bash
node sui/deploy-contract.js deploy Operators
```

Deploy the Abi package:

```bash
node sui/deploy-contract.js deploy Abi
```

Deploy the Governance package (requires `abi` and `axelar_gateway`):

```bash
node sui/deploy-contract.js deploy Governance
```

Deploy the ITS package (requires `abi`, `axelar_gateway` and `goverannce`):

```bash
node sui/deploy-contract.js deploy ITS
```

Deploy the Squid package (requires `abi`, `axelar_gateway`, `goverannce` and `its`):

```bash
node sui/deploy-contract.js deploy Squid
```

Call Contract:

```bash
node sui/gateway.js call-contract ethereum 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

Pay for gas:

The syntax is `node sui/gas-service.js payGas --amount <amount> <destinationChain> <destinationAddress> <channelId> <payload>`

```bash
node sui/gas-service.js payGas --amount 0.1 ethereum 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

Approve messages:

If the gateway was deployed using the wallet, you can submit a message approval with it

```bash
node sui/gateway.js approve --proof wallet --currentNonce test ethereum 0x0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-1 0x4F4495243837681061C4743b74B3eEdf548D56A5 0xa84d27bd6c9680e52e93779b8977bbcb73273b88f52a84d8dd8af1c3301341d7 0x47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad
```

Rotate gateway signers:

If gateway was deployed with the wallet as the verifier, and you want to rotate to the Amplifier verifiers, do

```bash
node sui/gateway.js rotate --proof wallet --currentNonce test
```

If you want to rotate to the wallet again but with a new nonce, do

```bash
node sui/gateway.js rotate --signers wallet --proof wallet --currentNonce test --newNonce test2
```

Use the same nonce for `--currentNonce` as the `--nonce` when deploying the gateway.

### Multisig

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
node sui/multisig.js --txBlockPath <path to unsigned tx block> --signatureFilePath <path to store signature> --action sign --offline
```

example txBlock file:

```
{
    "bytes": "AAACACBC5cSnnYJrDEn9nSW1BDzPLLAbUJbYOeJnUgYl/b90..."
}
```

Combine signature files:

```bash
node sui/multisig.js --txBlockPath <path to unsigned tx block> --signatureFilePath <path to store combined signature> --action combine --offline --signatures <paths to files containing signatures>
```

Execute combined signature:

This command will broadcast the signature to the network

```bash
node sui/multisig.js --txBlockPath <path to unsigned tx block> --action execute --combinedSignPath <path to combined signature>
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

GMP Demo:

Follow the instructions [here](docs/gmp.md)

## Troubleshooting

1. Move build error during the deployment step

Delete the `node_modules` folder and `package-lock.json` file and then run `npm install` again.

## Transfer object

Please note shared objects cannot be transferred via this script.

```bash
node sui/transfer-object.js --objectId <object id to be transferred> --recipient <recipient address>

node sui/transfer-object.js --contractName <Can be checked from config> --objectName <picked from config> --recipient <recipient address>
```

## Coins Management

List of coins in the wallet:

```bash
node sui/tokens.js list
```

Merge the coins:

```bash
node sui/tokens.js merge --coin-type <coin type to merge>
```

If coin type is not provided, it will merge all the coins.

Split the coins:

```bash
node sui/tokens.js split --amount <amount> --coin-type <coin type to split> --transfer <recipient address>
```

Note:
-   If coin type is not provided, it will split all the coins.
-   If transfer address is not provided, it will split the coins in the same wallet. Otherwise, it will transfer the splitted coins to the provided address.
