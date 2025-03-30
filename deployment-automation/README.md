# Axelar Deployment Setup

## Overview

This tool facilitates the deployment of Axelar network components by automating configurations, wallet setup, contract deployments, and governance integrations. It supports multiple environments, including `mainnet`, `testnet`, `stagenet`, `devnet-amplifier`, and custom `devnet` deployments.

The automation tool reduces the need for an operator to manually coordinate between deployment scripts, significantly streamlining the end-to-end deployment flow.

## Features

- Selectable network environments
- Environment-based configuration
- Automatic wallet creation
- Configuration file generation
- Contract deployment
- Governance and admin configurations
- Verifier and multisig setup

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourorg/axelar-deployment.git
   cd axelar-deployment
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
TARGET_CHAIN_PRIVATE_KEY=0xYourPrivateKeyHere
MNEMONIC="your twelve to twenty four word mnemonic phrase here"

# Optional Configuration
SERVICE_NAME=validators
VOTING_THRESHOLD=["6", "10"]
SIGNING_THRESHOLD=["6", "10"]
CONFIRMATION_HEIGHT=1
MINIMUM_ROTATION_DELAY=0
DEPLOYMENT_TYPE=create
```

**Important**: If you have environment variables already set in your shell that might conflict with those in your `.env` file, run the tool using:

```bash
env -u MNEMONIC -u TARGET_CHAIN_PRIVATE_KEY npm start
```

## Usage

### Running the Tool

To start the deployment process, run:

```bash
npm start
```

Follow the interactive prompts to guide the deployment.

### Network Selection

If `NAMESPACE` is not set in your `.env` file, you will be prompted to select a network:

- `mainnet`
- `testnet`
- `stagenet`
- `devnet-amplifier`
- Custom `devnet`

### Required Inputs

The tool requires the following details in your `.env` file:

- **Chain Name**: The name of the blockchain.
- **Chain ID**: The numerical ID for the chain.
- **Token Symbol**: Native token symbol.
- **Gas Limit**: Gas limit per transaction.
- **Private Key**: Private key for transaction signing. This key needs to be properly funded to deploy and interact with smart contracts on the target chain.
- **RPC URL**: Endpoint for blockchain interactions.
- **Axelar RPC URL**: Endpoint for Axelar node interactions.
- **Mnemonic**: Wallet mnemonic for key management. The deployer will need to have a properly funded Axelar wallet to perform deployments and proposals on the target Axelar network.

### Custom Network Requirements

For custom devnets, the corresponding config JSON is required and should be copied to the `axelar-chains-config/info` directory.

## Deployment Steps

Deployments are broken into three steps:

1. **Initial Deployment**: This step sets up the necessary configurations, deploys contracts, and generates the required environment files.

2. **Verifier Registration & Multisig Proposals**: After the initial deployment, external actions must take place:
   - Verifiers must register the chain with Axelar.
   - Once verifiers have registered the chain, multisig proposals must be created and submitted.
   - The proposals then require validation and approval before moving forward.

3. **Completing Deployment**: Once the verifiers have registered the chain and multisig proposals have been verified, the tool can proceed with finalizing the deployment steps. This includes confirming multisig contract interactions and ensuring the system is fully integrated.

## Deployment Configuration Storage

The tool uses a structured approach to store deployment configurations:

- All configurations are stored within the network's JSON file (e.g., `mainnet.json`, `testnet.json`)
- Each network file contains a `deployments` section with:
  - A `default` entry that stores network-wide configuration defaults
  - Individual chain entries stored under their respective chain names
- When starting a new deployment, the tool uses values from the `default` entry when not explicitly provided
- When resuming a deployment, it first loads the default values and then applies chain-specific overrides
- Sensitive data (private keys, mnemonics) is excluded from the stored configurations

This structure ensures consistent deployments while allowing for chain-specific customizations.

## Resuming Deployment

If deployment is interrupted, it can be resumed:

1. Run the tool:
   ```bash
   npm start
   ```

2. When prompted, select `no` to indicate this is not a new deployment:
   ```
   Is this a new deployment? (yes/no): no
   ```

3. The tool will prompt for the chain name and network if not already provided.

4. If multiple chain deployments exist for the selected network, you can choose which one to resume.

5. The tool will ask additional questions to determine where to resume:
   - Have verifiers registered support for the chain?
   - Have multisig proposals been approved?

Based on your answers, the tool will continue from the appropriate stage in the deployment process.

## Stopping Points

The entire deployment process is broken into multiple steps that account for asynchronous actions that must be coordinated.

### Verifier Updates

The first stopping point is after the Gateway registration proposal has been submitted and approved. Verifiers need to update their `config.toml` files and register chain support:

```bash
[[handlers]]
chain_finalization="RPCFinalizedBlock"
chain_name="$CHAIN"
chain_rpc_url=[http url]
cosmwasm_contract="$VOTING_VERIFIER"
type="EvmMsgVerifier"

[[handlers]]
chain_finalization="RPCFinalizedBlock"
chain_name="$CHAIN"
chain_rpc_url=[http url]
cosmwasm_contract="$VOTING_VERIFIER"
type="EvmVerifierSetVerifier"
```

```bash
ampd register-chain-support validators $CHAIN
```

### Multisig Proposals

After verifiers have registered support, the tool will verify and proceed to submit proposals for the multisig address. Since these proposals need to be approved, the tool will pause to let voting proceed.

### Reward Pools and Final Contract Execution

Once the multisig proposals are approved, the tool will create reward pools, set up the genesis verifier set, and deploy the gateway on the target chain.

## Security Considerations

- The `.env` file contains sensitive information and should never be committed to version control
- Private keys and mnemonics are only loaded from the `.env` file and are not written to deployment configuration files
- Always use a dedicated wallet/key for deployment operations
- Consider using remote signing solutions for production deployments

## Troubleshooting

If you encounter issues with environment variables not being loaded from your `.env` file:

1. Check for conflicting environment variables in your shell:
   ```bash
   env | grep MNEMONIC
   ```

2. Run the tool with explicitly unset variables:
   ```bash
   env -u MNEMONIC -u TARGET_CHAIN_PRIVATE_KEY npm start
   ```

## Error Handling

Common failure points include:

- **Invalid Private Key**: Ensure the private key starts with `0x`.
- **Invalid RPC URLs**: Must start with `http://` or `https://`.
- **Chain Already Exists in Config**: Prevents accidental overwrites.
- **Missing Environment Variables**: Check your `.env` file for completeness.