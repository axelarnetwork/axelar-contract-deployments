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
        "rpc": "[local rpc]",
        "contracts": {}
    }
}
```

Use the `-e local` (or `ENV=local` in the `.env` config) flag with scripts to run against the local network.

## Scripts

1. Faucet: To get test SUI coins to your address.

```bash
node sui/faucet.js
```

2. Deploy the gateway:

```bash
node sui/deploy-gateway.js -e testnet --signers '{"signers": [{"pubkey": "0x020194ead85b350d90472117e6122cf1764d93bf17d6de4b51b03d19afc4d6302b", "weight": 1}], "threshold": 1, "nonce": ""}'
```

3. Deploy the test GMP package:

```bash
node sui/deploy-test.js
```
