# Stacks Deployment Guide

## Table of Contents

-   [Prerequisites](#prerequisites)
-   [Deployment](#deployment)
-   [Contract Upgrades](#contract-upgrades)
-   [Contract Interactions](#contract-interactions)

## Prerequisites

A Stacks contracts deployer account can be created as follows. (if you don't have one already)

```bash
node stacks/generate-wallet.js
```

Set `STACKS_MNEMONIC="stacksmnemonic..."` in your `.env` file. If you have a private key you can set `STACKS_PRIVATE_KEY` instead.

## Deployment

To get test Stacks coins to your address via a faucet.

```bash
node stacks/faucet.js
```

In order to deploy all the contracts required by the Stacks integration, please follow the instructions from the [Stacks release document](../releases/stacks/2025-05-v1.0.0.md).

## Contract Interactions

### Collect Fees

```bash
node stacks/gas-service.js collect-fees --amount 1000000 --receiver <optional stacks receiver address>
```

### Setup Trusted Chain

Set trusted chian

```bash
node stacks/its.js set-trusted-chain <sourceChain> <sourceChain2> ...
```

or Add all chains that have ITS contract deployed

```bash
node stacks/its.js set-trusted-chain all
```

Remove trusted chain

```bash
node stacks/its.js remove-trusted-chain <sourceChain> <sourceChain2> ...
```

### Setup Governance

```bash
ts-node stacks/setup-contract.js Governance --governanceChain "[governance chain]" --governanceAddress "[governance address]"
```

### Set owner

```bash
ts-node stacks/commands.js set-owner <Gateway> "[governance address]"
```
