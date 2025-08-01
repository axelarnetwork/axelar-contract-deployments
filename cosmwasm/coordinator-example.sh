#!/bin/bash

# Coordinator Command Example Script
# This script demonstrates how to use the coordinator command

set -e  # Exit on any error

# Configuration
CHAIN_NAME="avalanche"
ENVIRONMENT="devnet-tkulik"


echo "=== Coordinator Command Example ==="
echo "Chain: $CHAIN_NAME"
echo "Environment: $ENVIRONMENT"
echo ""

# Check if mnemonic is provided
if [ "$MNEMONIC" = "your twelve word mnemonic phrase goes here" ]; then
    echo "ERROR: Please update the MNEMONIC variable in this script with your actual mnemonic phrase"
    echo "Example: MNEMONIC=\"word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12\""
    exit 1
fi

echo ""
echo "=== Example with Custom Parameters (Non-Default Values Only) ==="

# Example with custom parameters (only overriding defaults)
npx ts-node coordinator.ts \
    -n "$CHAIN_NAME" \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --service-name "validators" \
    --voting-threshold "6,10" \
    --signing-threshold "6,10" \
    --block-expiry "10" \
    --confirmation-height "1" \
    --salt "my-custom-salt-1" \
    --source-gateway-address "0x33eB9fF24a4b4A857a86264E8D26b6E7B9e9d421" \
    --governance-address "axelar18g6s82ll6cfv2v5ssm7x3dwcr9n9dgumcnphmm"

echo ""
echo "=== Example with Custom Governance Address ==="

# Example with custom governance address
npx ts-node coordinator.ts \
    -n "$CHAIN_NAME" \
    -e "$ENVIRONMENT" \
    -m "$MNEMONIC" \
    --governance-address "axelar1customgovernanceaddress" \
    --salt "custom-gov-deployment-$(date +%s)" \
    -y
