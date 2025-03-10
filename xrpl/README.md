
# XRPL deployments

## Installation

Install npm dependencies.

```sh
npm ci
```

Create a new XRPL keypair.

```bash
node xrpl/generate-wallet.js
```

Set `PRIVATE_KEY` in `.env` to the generated wallet's `seed` value.

Devnet and testnet funds can be obtained via the `faucet.js` script:

```bash
node xrpl/faucet.js -e devnet-amplifier -n xrpl-test-1
```

## XRPL Multisig Account Deployment

Deploy a new XRPL multisig account (the equivalent of the edge AxelarGateway on XRPL):

```bash
node xrpl/deploy-multisig.js -e <env> -n <chain-name> --initial-signer <xrpl-address>
```

This will transform the environment wallet into an XRPL multisig account and configure it appropriately.

## Rotate XRPL Multisig Signers

Rotate the XRPL multisig account's signer set (via a `SignerListSet` transaction):

```bash
node xrpl/rotate-signers.js -e <env> -n <chain-name> --signerPublicKeys <signer-public-keys> --signerWeights <signer-weights> --quorum <quorum>
```

The environment wallet must be the initial signer of the multisig account, with enough weight to reach quorum.

Here's an example signer set rotation:

```bash
node xrpl/rotate-signers.js -e testnet -n xrpl-test-1 --signerPublicKeys 028E425D6F75EC61C8568B7E1C29D3085E210A90A0CE6491E7A249747D34431F6C 02D904B083B855A5AE1DAB39ACE60227E110E0490AAA74DE18F5806121369DBB48 02F77F629E38433F6D2CE5EE46B7E8E1724444163FB08B99CF2C1B117A0E8578F1 0285737FE8BA5D8E8F2A10CB39E814D5E72DADF8FF05BDFABCCF1EF20C51279EC8 --signerWeights 1 1 1 1 --quorum 3
```

## GMP/ITS Transfers

GMP and/or ITS transfers can be performed via the `transfer.js` script:

```bash
node xrpl/transfer.js -e <env> -n <source-chain> [token] [amount] [destination-chain] [destination-address] --gas-fee-amount [gas-fee-amount] --payload [payload]
```

Here's an example of a token transfer that also performs GMP:

```bash
node xrpl/transfer.js -e devnet-amplifier -n xrpl-test-1 XRP 1 xrpl-evm-sidechain 0x0A90c0Af1B07f6AC34f3520348Dbfae73BDa358E --payload 0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000e474d5020776f726b7320746f6f3f000000000000000000000000000000000000
```

## Broadcast Raw Transaction Blob

Broadcast a raw, signed transaction blob (e.g., a completed Multisig Prover proof) to XRPL:

```bash
node xrpl/broadcast.js -e testnet -n xrpl-test-1 [blob]
```

## Decode XRPL Address

Convert an XRPL Address (aka Account ID) into raw bytes (e.g., to be used when specifying an XRPL destination address on ITS):

```bash
node xrpl/decode-address.js [account-id]
```

Here's an example:

```bash
node xrpl/decode-address.js r9m9uUCAwMLSnRryXYuUB3cGXojpRznaAo
# Account ID raw bytes: 0x601abcea746a193f32ed460dd933f15441142d6b
```

## Decode Raw Transaction Blob

Deserialize a raw transaction blob into a readable transaction object:

```bash
node xrpl/decode-tx-blob.js <tx-blob>
```

Here's a truncated example:

```bash
node xrpl/decode-tx-blob.js 120000220000000024000000002029004c6ce7614[...]738623034353436656239322d3732393935e1f1
# Decoded transaction: {
#   "TransactionType": "Payment",
#   "Flags": 0,
#   "Sequence": 0,
#   "TicketSequence": 5008615,
#   "Amount": "1000000",
#   "Fee": "5000",
#    [...]
# }
```

## Create a Trust Line

Create a trust line between your account and a token issuer (via a TrustSet transaction).

```bash
node xrpl/trust-set.js -e <env> -n <chain-name> [token-currency] [token-issuer-address]
```

Here's an example:

```bash
node xrpl/trust-set.js -e devnet-amplifier -n xrpl-test-1 XYZ r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1
```
