# Operators Contract on Sui

This document provides a step-by-step guide to deploy and use the Operators contract on the Sui network.

## Setup

### Prerequisites

1. Fork and clone the [Axelar Contract Deployments](https://github.com/axelarnetwork/axelar-contract-deployments) repository

2. Install dependencies and build the project

```bash
cd axelar-contract-deployments
npm ci && npm run build
```

3. Install [ts-node](https://www.npmjs.com/package/ts-node) globally

```bash
npm install -g ts-node
```

### Environment Configuration

1. Create an environment file and add your contract admin key information there

```bash
touch .env
```

2. Add the following environment variables to the `.env` file:

```bash
# Setup ENV
ENV="testnet"

# Your Sui private key
PRIVATE_KEY="suiprivkey1..."

# Signature scheme
SIGNATURE_SCHEME="secp256k1"
```

### Synchronize Contracts

Synchronize the various contract deployments by executing the Sui contracts sync command:

```bash
ts-node sui/deploy-contract sync
```

## Deployment

Deploy the Operators contract using the following command:

```bash
ts-node sui/deploy-contract deploy operators
```

## Usage

### Add Operators

Add operators to your contract using the following command:

```bash
ts-node sui/operators add <sui address> [--ownerCap <ownerCapId>]
```

### Remove Operator

Remove an operator from your contract:

```bash
ts-node sui/operators remove <sui address> [--ownerCap <ownerCapId>]
```

### Store Treasury Cap

Store the `TreasuryCap` using the following command:

```bash
ts-node sui/operators storeCap --capId <treasuryCapId>
```

### Remove Cap

Remove a previously stored capability:

```bash
ts-node sui/operators removeCap <capId> [--ownerCap <ownerCapId>] [--receiver <address>]
```

### Collect Gas

Collect gas from the gas service for the operator (receiver optional):

```bash
ts-node sui/operators collectGas --amount <amount> [--receiver <address>]
```

### Refund Gas

Refund gas for a specific message (receiver optional):

```bash
ts-node sui/operators refund <messageId> --amount <amount> [--receiver <address>]
```

## TODO

Write a script to perform your desired operation / move calls, ensuring that the move call is executed within the same transaction between `loanCap` and `restoreCap`. 
