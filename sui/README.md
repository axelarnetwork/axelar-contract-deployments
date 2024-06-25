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
        "contracts": {}
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

- By querying the signer set from the Amplifier contract (this only works if Amplifier contracts have been setup):

```bash
node sui/deploy-gateway.js
```

Use `--help` flag to see other setup params that can be overridden.

- For testing convenience, you can use the secp256k1 wallet as the signer set for the gateway.

```bash
node sui/deploy-gateway.js --signers wallet --nonce test
```

- You can also provide a JSON object with a full signer set:

```bash
node sui/deploy-gateway.js -e testnet --signers '{"signers": [{"pubkey": "0x020194ead85b350d90472117e6122cf1764d93bf17d6de4b51b03d19afc4d6302b", "weight": 1}], "threshold": 1, "nonce": "0x0000000000000000000000000000000000000000000000000000000000000000"}'
```

Deploy the test GMP package:

```bash
node sui/deploy-test.js
```

Call Contract:

```bash
node sui/gateway.js call-contract ethereum 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

Approve messages:

```bash
node sui/gateway.js approve ethereum 0x0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-1 0x4F4495243837681061C4743b74B3eEdf548D56A5 0xa84d27bd6c9680e52e93779b8977bbcb73273b88f52a84d8dd8af1c3301341d7 0x47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad --proof wallet
```

Rotate gateway signers:

If gateway was deployed with the wallet as the verifier, and you want to rotate to the Amplifier verifiers, do

```bash
node sui/gateway.js rotate --proof wallet
```

If you want to rotate to the wallet again but with a new nonce, do

```bash
node sui/gateway.js rotate --signers wallet --proof wallet --currentNonce test --newNonce test2
```

Use the same nonce for `--currentNonce` as the `--nonce` when deploying the gateway.


### Multisig

Sign transaction block for multisig:

```bash
node sui/multisig.js -p <private key> --signatureScheme <signer scheme type> --file <file path where unsigned tx block data is stored> --txFile <file path where signed signature will be stored> --action sign --offline <true/false: to store signature or execute transaction>
```
example file:
```
{
    "bytes": "AAADAQFQT+fE+oFTHe629c2CKmmnBk6F8FJ/q0IcsJHXGKg3Uv1rtAIAAAAAAQAIGAAAAAAAAAAAIAv8A6qCEJTg18khmS29w7e6lW6MM3iP4elh2nWghSoqAQCDSROiLpOQ9qB5p9Z5yb4iaSHeu9+FedwD7BmZ8nAXVwZteWNvaW4EbWludAADAQAAAQEAAQIAhBIyNmelLibtcBkL+mbrAtKsFpOXYdhWU6dha9IfnHkBqw18qBGpceHqCT2ZlhjFEfovQoT3Y39LvgOk6fwj0ncJbLQCAAAAACC9XSxZW+G2Q1wVrkKlVQfOVJLTnQXj5Ckl+CBqghszS4QSMjZnpS4m7XAZC/pm6wLSrBaTl2HYVlOnYWvSH5x56AMAAAAAAADwf0IAAAAAAAA="
}
```

Combine signature files:

```bash
node sui/multisig.js -p <private key> --signatureScheme <signer scheme type> --file <file path where unsigned tx block data is stored> --txFile <file path where sign signature will be stored> --action combine --offline <true/false: to store signature or execute transaction> --signatures <path to files containing signed tx data>
```
example signature:
```
{
  "signature": "AMATYcqvEwn1uHLaXkTSpL/yoTC3esxnmf81JSSe3OGB17TaWlWMJRS0fAyWp8RGhIeTpq/afCmOnQGrby7QBghxAMFV+C3YAVScXJlpEn13dBKtHsBZULxX8eIVyr6FHw==",
  "publicKey": "0x7100c155f82dd801549c5c9969127d777412ad1ec05950bc57f1e215cabe851f",
  "message": "AAADAQFQT+fE+oFTHe629c2CKmmnBk6F8FJ/q0IcsJHXGKg3Uv1rtAIAAAAAAQAIGAAAAAAAAAAAIAv8A6qCEJTg18khmS29w7e6lW6MM3iP4elh2nWghSoqAQCDSROiLpOQ9qB5p9Z5yb4iaSHeu9+FedwD7BmZ8nAXVwZteWNvaW4EbWludAADAQAAAQEAAQIAhBIyNmelLibtcBkL+mbrAtKsFpOXYdhWU6dha9IfnHkBqw18qBGpceHqCT2ZlhjFEfovQoT3Y39LvgOk6fwj0ncJbLQCAAAAACC9XSxZW+G2Q1wVrkKlVQfOVJLTnQXj5Ckl+CBqghszS4QSMjZnpS4m7XAZC/pm6wLSrBaTl2HYVlOnYWvSH5x56AMAAAAAAADwf0IAAAAAAAA="
}
```

Execute combined signature:

```bash
node sui/multisig.js -p <private key> --signatureScheme <signer scheme type> --file <file path where unsigned tx block data is stored> --txFile <file path where signed signature will be stored> --action execute --offline <true/false: to store signature or execute transaction> --multisigTxFile <file path where combine signature of tx data is stored of multisig>
```

example combined signature:
```
{
  "signature": "AwIAwBNhyq8TCfW4ctpeRNKkv/KhMLd6zGeZ/zUlJJ7c4YHXtNpaVYwlFLR8DJanxEaEh5Omr9p8KY6dAatvLtAGCABAwaSKOLq7ozJ/HNtrK/YNHcJ4op1DaKnNKqplwK9nfPiSKO+j26zs/Hm784p38PWBqvooTFa42FDZV4raXCgHAwACAHEAwVX4LdgBVJxcmWkSfXd0Eq0ewFlQvFfx4hXKvoUfAQC2FlZWh/1t33DD+4nNxx1SekhnK/M4n1kiRO1fxPuDzgECAA==",
  "status": "PENDING",
  "message": "AAADAQFQT+fE+oFTHe629c2CKmmnBk6F8FJ/q0IcsJHXGKg3Uv1rtAIAAAAAAQAIGAAAAAAAAAAAIAv8A6qCEJTg18khmS29w7e6lW6MM3iP4elh2nWghSoqAQCDSROiLpOQ9qB5p9Z5yb4iaSHeu9+FedwD7BmZ8nAXVwZteWNvaW4EbWludAADAQAAAQEAAQIAhBIyNmelLibtcBkL+mbrAtKsFpOXYdhWU6dha9IfnHkBqw18qBGpceHqCT2ZlhjFEfovQoT3Y39LvgOk6fwj0ncJbLQCAAAAACC9XSxZW+G2Q1wVrkKlVQfOVJLTnQXj5Ckl+CBqghszS4QSMjZnpS4m7XAZC/pm6wLSrBaTl2HYVlOnYWvSH5x56AMAAAAAAADwf0IAAAAAAAA="
}
```

use --multisigKey `multisigKey` to override existing multisig info in chains config.

## Troubleshooting

1. Move build error during the deployment step

Delete the `node_modules` folder and `package-lock.json` file and then run `npm install` again.
