# Sui deployment scripts

## Prerequisites

A private key can be created as follows:

```bash
sui client new-address secp256k1 wallet

# Export private key in bech32 format
sui keytool export --key-identity wallet
```

Set `PRIVATE_KEY=[suiprivkey...]` in your `.env` file. Other private key types are supported via `--privateKeyType` and `--signatureScheme` flags.

## Scripts

1. Faucet: To get test SUI coins to your address.

`node sui/faucet.js`

2. Deploy gateway:

`node sui/deploy-gateway.js --signers '{"signers": [{"pubkey": "0x9076afa53e8a21bcf01b3e99b93044e4a005fcab5351f4ea5f72a34e31e0d5fc00", "weight": 1}], "threshold": 1, "nonce": ""}'`
