
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
