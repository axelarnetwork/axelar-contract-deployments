# Coordinator Script Documentation

The Coordinator script is a TypeScript-based CLI tool for managing the deployment and instantiation of Axelar Amplifier contracts on Axelar chain. It provides a streamlined workflow for deploying and configuring the core contracts needed for the Axelar Amplifier system.

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
GOVERNANCE_ADDRESS="<axelar governance address>"
SRC_GATEWAY_ADDRESS="<external chain gateway address>"
CONTRACT_ADMIN_ADDRESS="<axelar contracts' admin address>"

# Step 1: Register protocol (if not already done)
npx ts-node cosmwasm/coordinator.ts register-protocol \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --run-as "$GOVERNANCE_ADDRESS"

# Step 2: Deploy contracts
npx ts-node cosmwasm/coordinator.ts deploy \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --run-as "$GOVERNANCE_ADDRESS" \
    --contract-admin "$CONTRACT_ADMIN_ADDRESS" \
    --artifact-dir "./artifacts/"

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
    --governance-address "$GOVERNANCE_ADDRESS" \
    --run-as "$GOVERNANCE_ADDRESS"

# Step 4: Register deployment
npx ts-node cosmwasm/coordinator.ts register-deployment \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --run-as "$GOVERNANCE_ADDRESS" \
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

## Design Discussion

The procedure of the direct deployment without governance proposal is not supported. The reasoning:
1. We have to add instantiate permissions for `Gateway`, `VotingVerifier` and `Multisig` to allow `Coordinator` to instantiate them.
2. We can do that when deploying the contracts using gov proposals - the `deploy` command does that.
3. We can't add the permissions when uploading contract directly (without gov proposals) - it's unauthorized operation even for the gov address as a sender.

Since we have to vote for the proposals anyway, there's no point in maintaining a flow where we first deploy contracts directly and then send a gov proposal to update the instantiate permissions lists.

## Security Considerations

1. **Mnemonic Security**: Never hardcode mnemonics in scripts. Use environment variables or secure input methods.
2. **Environment Variables**: Set the `MNEMONIC` environment variable for automated deployments.
3. **Governance Proposals**: All deployments require governance approval, ensuring proper oversight.
4. **Confirmation Prompts**: Use `-y` flag to skip confirmation prompts in automated environments.


## Troubleshooting

1. **Invalid Mnemonic**: Ensure the mnemonic phrase is correct and has sufficient funds.
2. **Network Issues**: Check RPC endpoint connectivity.
3. **Insufficient Funds**: Ensure the signing address has enough tokens for gas and deposits.
4. **Contract Artifacts**: Verify that contract WASM files are available in the specified directory.
5. **Proposals not accepted**: All proposals need to be voted on and the voting period needs to be finished before proceeding to the next step.
