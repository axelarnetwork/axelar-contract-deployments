# Coordinator Script Documentation

The Coordinator script is a TypeScript-based CLI tool for managing the deployment and instantiation of Axelar Amplifier contracts on Cosmos chains. It provides a streamlined workflow for deploying and configuring the core contracts needed for the Axelar Amplifier system.

## Overview

The Coordinator script manages the deployment of three main contract types:
- **VotingVerifier**: Handles voting verification for cross-chain messages
- **MultisigProver**: Manages multisignature proof verification
- **Gateway**: Acts as the entry point for cross-chain communication

## Prerequisites

1. **Node.js and npm**: Ensure you have Node.js installed
2. **TypeScript**: The script uses ts-node for execution
3. **Mnemonic**: A valid mnemonic phrase for signing transactions
4. **Contract Artifacts**: Compiled WASM files for the contracts

## Installation

```bash
# Install dependencies
npm ci && npm run build

# Ensure ts-node is available
npm install -g ts-node
```

## Complete Workflow Example

Here's a complete example workflow for deploying and configuring contracts for a new chain:

```bash
#!/bin/bash

# Configuration
CHAIN_NAME="some-network"
ENVIRONMENT="devnet-custom"
MNEMONIC="your twelve word mnemonic phrase goes here"
RUN_AS_ADDRESS="<axelar gov & admin>"
SRC_GATEWAY_ADDRESS="<external chain gateway address>"

# Step 1: Register protocol (if not already done)
npx ts-node cosmwasm/coordinator.ts register-protocol \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --run-as "$RUN_AS_ADDRESS"

# Step 2: Deploy contracts
npx ts-node cosmwasm/coordinator.ts deploy \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --run-as "$RUN_AS_ADDRESS" \
    --artifact-dir "./artifacts/"

# Step 2a: Allow Coordinator contract to instantiate Gateway, MultisigProver and VotingVerifier
# NOTE: This is needed only when using `deploy` command with `--direct` flag enabled.
npx ts-node cosmwasm/coordinator.ts update-instantiate-config \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --run-as "$RUN_AS_ADDRESS"

# Step 3: Instantiate contracts with custom parameters
npx ts-node cosmwasm/coordinator.ts instantiate \
    -n "$CHAIN_NAME" \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --service-name "validators" \
    --voting-threshold "6,10" \
    --signing-threshold "6,10" \
    --block-expiry "10" \
    --confirmation-height "1" \
    --source-gateway-address "$SRC_GATEWAY_ADDRESS" \
    --governance-address "$RUN_AS_ADDRESS" \
    --run-as "$RUN_AS_ADDRESS"

# Step 4: Register deployment
npx ts-node cosmwasm/coordinator.ts register-deployment \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --run-as "$RUN_AS_ADDRESS" \
    -n "$CHAIN_NAME"
```

## Default Values

The script uses the following default values:

- **Service Name**: "amplifier"
- **Voting Threshold**: ["51", "100"]
- **Signing Threshold**: ["51", "100"]
- **Block Expiry**: "10"
- **Confirmation Height**: 1000000
- **Message ID Format**: "hex_tx_hash_and_event_index"
- **Address Format**: "eip55"
- **Verifier Set Diff Threshold**: 1
- **Encoder**: "abi"
- **Key Type**: "ecdsa"
- **Proposal Deposit**: "1000000000"

## Security Considerations

1. **Mnemonic Security**: Never hardcode mnemonics in scripts. Use environment variables or secure input methods.
2. **Environment Variables**: Set the `MNEMONIC` environment variable for automated deployments.
3. **Direct Execution**: Use the `--direct` flag carefully as it bypasses governance proposals.
4. **Confirmation Prompts**: Use `-y` flag to skip confirmation prompts in automated environments.

## Error Handling

The script includes comprehensive error handling:
- Input validation for all parameters
- Network connectivity checks
- Transaction confirmation verification
- Retry logic for failed operations
- Detailed error messages for debugging

## Troubleshooting

1. **Invalid Mnemonic**: Ensure the mnemonic phrase is correct and has sufficient funds.
2. **Network Issues**: Check RPC endpoint connectivity.
3. **Insufficient Funds**: Ensure the signing address has enough tokens for gas and deposits.
4. **Contract Artifacts**: Verify that contract WASM files are available in the specified directory.
5. **Proposals not accepted**: When `--direct` flag is not set, the proposals are submitted.
   Each proposal needs to be voted and the voting period needs to be finished before going to the next step.
