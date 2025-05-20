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
node stacks/gas-service.js collect-fees --amount 1000000 --receiver <receiver address>
```

## Setup Trusted Address

Set trusted address

```bash
node stacks/its.js set-trusted-address <sourceChain> <sourceChain2> ...
```

or Add all chains that have ITS contract deployed

```bash
node stacks/its.js set-trusted-address all
```

Remove trusted addresses

```bash
node stacks/its.js remove-trusted-address <sourceChain> <sourceChain2> ...
```
