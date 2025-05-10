# Test GMP Flow on Sui

This document provides a step-by-step guide to execute GMP flow on the SUI network.

## Table of Contents

1. [Setup the Environment](#setup-the-environment)
2. [Send Command (Outgoing)](#send-command-outgoing)
3. [Execute Command (Incoming)](#execute-command-incoming)

## Setup the Environment

Deploy the contracts with the following commands:

### Contract Deployment

```bash
npm run script sui/faucet
npm run script sui/deploy-contract deploy Utils
npm run script sui/deploy-contract deploy VersionControl
npm run script sui/deploy-contract deploy AxelarGateway --signers wallet
npm run script sui/deploy-contract deploy GasService
npm run script sui/deploy-contract deploy Abi
npm run script sui/deploy-contract deploy RelayerDiscovery
npm run script sui/deploy-contract deploy ITS
npm run script sui/deploy-contract deploy Example
```

### Prepare Parameters

To simplify the process of obtaining necessary parameters, run the following script:

```bash
sourceChain=avalanche-fuji
sourceAddress=0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5
messageId=0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-10
fee=0.1
payload=0x1234
payloadHash=0x56570de287d73cd1cb6092bb8fdee6173974955fdef345ae579ee9f475ea7432
env=$(grep ENV= .env | cut -d '=' -f2 | tr -d '\n')
config=$(cat "axelar-chains-config/info/${env}.json")
destinationId=$(echo $config | jq -r '.sui.contracts.Example.objects.GmpChannelId')
```

## Send Command (Outgoing)

Send a messsage from SUI to the destination chain.

Example:

```bash
npm run script sui/gmp.js sendCall $sourceChain $sourceAddress $fee $payload
```

## Execute Command (Incoming)

Execute a message from the source chain at SUI application module.

1. Approve the incoming message

```bash
npm run script sui/gateway.js approve --proof wallet $sourceChain $messageId $sourceAddress $destinationId $payloadHash
```

2. Execute the incoming message

This command will execute the message to the deployed test contract.

```bash
npm run script sui/gmp.js execute $sourceChain $messageId $sourceAddress $payload
```
