# Test ITS Flow on Sui

This document will guide you through testing the ITS (Interchain Token Service) flow on Sui. The process includes setting up the environment, sending tokens between chains, and deploying tokens.

## Table of Contents

1. [Setup the Environment](#setup-the-environment)
    - [Contract Deployment](#contract-deployment)
    - [Prepare Parameters](#prepare-parameters)
    - [Deploy Test Tokens](#deploy-test-tokens)
    - [Setup Trusted Addresses](#setup-trusted-addresses)
2. [Token Transfer](#token-transfer)
    - [Send Token to Gateway](#send-token-to-gateway)
    - [Receive Token](#receive-token)
3. [Token Deployment](#token-deployment)
    - [Send Token Deployment to Gateway](#send-token-deployment-to-gateway)
    - [Receive Token Deployment](#receive-token-deployment)

## Setup the Environment

### Contract Deployment

Deploy the contracts with the following commands:

```bash
node sui/faucet
node sui/deploy-contract deploy Utils
node sui/deploy-contract deploy VersionControl
node sui/deploy-contract deploy AxelarGateway --signers wallet
node sui/deploy-contract deploy GasService
node sui/deploy-contract deploy Abi
node sui/deploy-contract deploy RelayerDiscovery
node sui/deploy-contract deploy ITS
node sui/deploy-contract deploy Example
```

### Prepare Parameters

To simplify the process of obtaining necessary parameters, run the following script:

```bash
sourceChain=Ethereum
transferMessageId=0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-01
deployMessageId=0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-02
sourceAddress=0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5
amount=1
symbol=IMD
decimals=9
name=interchain-moo-deng
emptyTokenSymbol=ETY
emptyTokenName=Empty
emptyTokenDecimals=6
config=$(cat axelar-chains-config/info/local.json)
channelId=$(echo $config | jq -r '.sui.contracts.ITS.objects.ChannelId')
destinationContractAddress=$(echo $config | jq -r '.sui.contracts.Example.objects.ItsChannelId')
```

### Deploy Test Tokens

```bash
node sui/its-example deploy-token $symbol $name $decimals
node sui/its-example deploy-token $emptyTokenSymbol $emptyTokenName $emptyTokenDecimals
```

This command modifies the token template in the `interchain_token.move` file with the given name, symbol, and decimals. It then deploys the modified token contract and stores the Metadata, TreasuryCap, and TokenId in the config. Finally, it registers the coin on the ITS contract.

Prepare additional parameters for the example:

```bash
transferInfo=$(node sui/its-example.js print-receive-transfer $symbol $sourceAddress $amount)
transferPayloadHash=$(echo $transferInfo | jq -r .payloadHash)
deploymentInfo=$(node sui/its-example.js print-receive-deployment $emptyTokenName $emptyTokenSymbol $emptyTokenDecimals)
deployPayload=$(echo $deploymentInfo | jq -r .payload)
deployPayloadHash=$(echo $deploymentInfo | jq -r .payloadHash)
```

### Setup Trusted Addresses

The trusted address is used to verify the message both incoming and outgoing directions.

```bash
node sui/its-example.js setup-trusted-address $sourceChain $sourceAddress
```

## Token Transfer

### Send Token to Gateway

```bash
node sui/its-example send-token $symbol $sourceChain $sourceAddress 0.1 10
```

### Receive Token

1. Approve the gateway to execute the transfer:

```bash
node sui/gateway.js approve --proof wallet $sourceChain $transferMessageId $sourceAddress $channelId $transferPayloadHash
```

2. Receive the token:

```bash
node sui/its-example receive-token $sourceChain $transferMessageId $sourceAddress $symbol $amount
```

## Token Deployment

### Send Token Deployment to Gateway

```bash
node sui/its-example send-deployment $symbol $sourceChain $sourceAddress 0.1 10
```

### Receive Token Deployment

In this example, we'll use the empty token to receive the deployment because the token must have zero supply.

1. Approve the gateway to execute the deployment:

```bash
node sui/gateway.js approve --proof wallet $sourceChain $deployMessageId $sourceAddress $channelId $deployPayloadHash
```

2. Receive the token deployment:

```bash
node sui/its-example receive-deployment $emptyTokenSymbol $sourceChain $deployMessageId $sourceAddress $channelId $deployPayload
```
