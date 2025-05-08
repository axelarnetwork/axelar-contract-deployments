# Axelar Deployment Setup

## Overview

This tool facilitates the deployment of Axelar network components by automating configurations, wallet setup, contract deployments, and governance integrations. It supports multiple environments, including `mainnet`, `testnet`, `stagenet`, `devnet-amplifier`, and custom `devnet` deployments.

The automation tool reduces the need for an operator to manually coordinate between deployment scripts, significantly streamlining the end-to-end deployment flow. It is designed for both manual execution and automation via CI/CD pipelines.

## Features

- Command-line based automation
- Environment-based configuration
- Automatic wallet creation
- Configuration file generation
- Contract deployment
- Governance and admin configurations
- Verifier and multisig setup
- Error handling and validation

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/axelarnetwork/axelar-contract-deployments.git
   cd axelar-contract-deployments/deployment-automation
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Build the project:
   ```bash
   npm run build
   ```

## Environment Configuration

The tool uses a `.env` file for configuration. Create your environment file:

```bash
cp .env.example .env
```

Edit the `.env` file with your specific values:

```properties
# Chain Configuration
CHAIN_NAME=mychainnew
CHAIN_ID=43113
TOKEN_SYMBOL=AVAX
GAS_LIMIT=15000000

# Network RPC URLs
RPC_URL=https://avalanche-fuji-c-chain-rpc.publicnode.com
AXELAR_RPC_URL=http://k8s-devnetma-coresent-f604372d89-fc213dff8d4139da.elb.us-east-2.amazonaws.com:26657

# Network Selection
NAMESPACE=devnet-markus

# Sensitive Data (KEEP SECURE!)
PRIVATE_KEY=0xYourPrivateKeyHere
MNEMONIC="your twelve to twenty four word mnemonic phrase here"

# Optional Configuration
SERVICE_NAME=validators
VOTING_THRESHOLD=["6", "10"]
SIGNING_THRESHOLD=["6", "10"]
CONFIRMATION_HEIGHT=1
MINIMUM_ROTATION_DELAY=0
DEPLOYMENT_TYPE=create
VOTING_VERIFIER_CONTRACT_VERSION=1.1.0
GATEWAY_CONTRACT_VERSION=1.1.1
MULTISIG_PROVER_CONTRACT_VERSION=1.1.1
```

**Important**: If you have environment variables already set in your shell that might conflict with those in your `.env` file, run the tool using:

```bash
env -u MNEMONIC -u PRIVATE_KEY npm start
```

## Usage

### Running the Tool

The tool is now fully command-line based and requires specific flags for operation:

```bash
npm start -- [options]
```

### Command-Line Options

The following options are available:

```
Main Options:
  --new-deployment                Start a new deployment
  --resume-deployment             Resume an existing deployment
  
Resume Options:
  --verifiers-registered          Indicate verifiers have registered support
  --no-verifiers-registered       Indicate verifiers have not registered support
  --proposals-approved            Indicate multisig proposals have been approved
  --no-proposals-approved         Indicate multisig proposals have not been approved
  --force-gateway-deployment      Try to deploy gateway even if earlier steps fail
  --continue-on-error             Continue execution despite errors

Configuration Options:
  --namespace <value>             Set the network namespace
  --chain-name <value>            Set the chain name
  --chain-id <value>              Set the chain ID
  --token-symbol <value>          Set the token symbol
  --gas-limit <value>             Set the gas limit
  --rpc-url <value>               Set the RPC URL
  --axelar-rpc-url <value>        Set the Axelar RPC URL
  --version <value>               Set the contract version
  --help                          Display help information
```

### Deployment Process

The deployment process is split into two distinct stages:

1. **Initial Deployment**
   ```bash
   npm start -- --new-deployment
   ```
   This instantiates contracts and submits proposals to register chain with Axelar, authorize multisig prover, and create reward pools.

2. **After Verifiers Register and Proposals Are Approved**
   ```bash
   npm start -- --resume-deployment --chain-name mynewchain --verifiers-registered --proposals-approved
   ```
   This completes the deployment after multisig proposals have been approved.

### Error Handling and Recovery

The tool has built-in error handling for common scenarios:

- **Reward Pools Already Exist**: Automatically detected and handled
- **Verifier Set Not Changed**: Automatically detected and handled
- **Gateway Address Mismatch**: Throws error on production networks, continues on custom devnets
- **Gateway Already Registered**: Continues deployment if proposal for registering chain on gateway is already approved

### Stopping Points

The deployment process has predetermined stopping points that require external actions:

1. **After Initial Deployment**: Verifiers need to register support and proposals need to be voted on and approved
2. **After Verifier Registration**: In case proposals expire they can be resubmitted
3. **After Proposal Approval**: Final deployment steps

At each stopping point, the tool will print clear instructions for the next command to run.

## Deployment Configuration Storage

Deployment configurations are stored within the network's JSON file:

- Each network file (`mainnet.json`, `testnet.json`, etc.) contains a `deployments` section
- A `default` entry stores network-wide configuration defaults
- Individual chain deployments are stored under their respective chain names
- Sensitive data is excluded from stored configurations

## CI/CD Integration

Example GitHub Actions workflow that we want to achieve
1. Execute voting on submited proposals for devnet, testnet, stagenet
2. After initial deployment utilize generated configs to create PR or directly update infrastructure repo to update verifier set
3. Register the ampd support for chain
4. Config updates on the axelar-deployment-contracts repo for the axelar-chains-config needs to be commited to repository

## Security Considerations

- The `.env` file contains sensitive information and should never be committed to version control
- Private keys and mnemonics are only loaded from the `.env` file and are not written to deployment configuration files
- Always use a dedicated wallet/key for deployment operations
- Consider using remote signing solutions for production deployments

## Troubleshooting

If you encounter issues:

1. Check for conflicting environment variables in your shell:
   ```bash
   env | grep MNEMONIC
   ```

2. Run the tool with explicitly unset variables:
   ```bash
   env -u MNEMONIC -u PRIVATE_KEY npm start
   ```

3. For gateway deployment issues on non-custom networks, ensure the predicted address matches existing deployments by checking the salt, contract bytecode, and deployer address.

4. Use the `--help` flag to see all available options:
   ```bash
   npm start -- --help
   ```