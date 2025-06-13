# Starknet Deployment Scripts 🏗️

This directory contains deployment and operational scripts for Axelar contracts on Starknet. The scripts support both online and offline workflows, with hardware wallet integration for secure mainnet deployments.

## 🔧 Setup

### Prerequisites

- Node.js >= 18
- Starknet.js dependencies
- For mainnet: Ledger hardware wallet

### Installation

```bash
npm ci && npm run build
```

### TypeScript Support

All scripts now use `ts-node` for TypeScript execution:
- Compatible with existing JavaScript workflows
- Enhanced type safety and development experience
- All commands use `ts-node` instead of `node`

### Environment Configuration

Create a `.env` file with the following variables (see `.example.env` for reference):

```bash
# Starknet Configuration
STARKNET_PRIVATE_KEY=0x...  # For testnet/devnet only
STARKNET_ACCOUNT_ADDRESS=0x...

# Network settings
ENV=testnet  # or mainnet, devnet, stagenet
```

## 🚀 Core Features

### Dual Workflow Support
- **Online Mode**: Direct transaction execution (testnet/devnet)
- **Offline Mode**: Unsigned transaction generation for hardware wallet signing (mainnet)

### Chain Configuration
- Starknet scripts automatically use the 'starknet' chain from your environment config
- No need to specify chain names in commands

### Security Model
- **Testnet/Devnet**: Private key-based signing
- **Mainnet**: Mandatory offline workflow with Ledger hardware wallets

### Transaction Types
- **Invoke Transactions**: Contract calls and deployments
- **Declare Transactions**: Contract class declarations (online only)

### Contract Support
- ✅ Contract declaration, deployment and upgrades
- ✅ Gateway operations (call contract, approve messages, validate messages)
- ✅ Signer rotation and operatorship management
- 🔄 Additional contracts (Gas Service, Operators, ITS) - *coming soon*

## 📚 Core Workflow

### 1. Declare Contract (Online Only)
```bash
npx ts-node starknet/declare-contract.ts \
  --env testnet \
  --contractConfigName AxelarGateway \
  --contractPath ./artifacts/AxelarGateway.contract_class.json \
  --privateKey 0x... \
  --accountAddress 0x...
```

This will declare the contract on-chain and save the class hash to the configuration.

### 2. Deploy Contract

**Online Deployment (Testnet/Devnet):**
```bash
npx ts-node starknet/deploy-contract.ts \
  --env testnet \
  --contractConfigName AxelarGateway \
  --constructorCalldata '["0x1234"]' \
  --salt 0x123 \
  --privateKey 0x... \
  --accountAddress 0x...
```

**Offline Deployment (Mainnet):**
```bash
# 1. Estimate gas (online environment)
npx ts-node starknet/deploy-contract.ts \
  --env mainnet \
  --contractConfigName AxelarGateway \
  --constructorCalldata '["0x1234"]' \
  --salt 0x123 \
  --estimate \
  --privateKey 0x... \
  --accountAddress 0x...

# 2. Generate unsigned transaction (offline environment)
npx ts-node starknet/deploy-contract.ts \
  --env mainnet \
  --contractConfigName AxelarGateway \
  --constructorCalldata '["0x1234"]' \
  --salt 0x123 \
  --offline \
  --nonce 5 \
  --accountAddress 0x... \
  --l1GasMaxAmount 50000 \
  --l1GasMaxPricePerUnit 100000000000 \
  --l2GasMaxAmount 1000000 \
  --l2GasMaxPricePerUnit 1000000000
```

### 3. Upgrade Contract

**Online Upgrade:**
```bash
npx ts-node starknet/upgrade-contract.ts \
  --env testnet \
  --contractConfigName AxelarGateway \
  --classHash 0xNewClassHash... \
  --privateKey 0x... \
  --accountAddress 0x...
```

**Offline Upgrade:**
```bash
# 1. Estimate gas (online environment)
npx ts-node starknet/upgrade-contract.ts \
  --env mainnet \
  --contractConfigName AxelarGateway \
  --classHash 0xNewClassHash... \
  --estimate \
  --privateKey 0x... \
  --accountAddress 0x...

# 2. Generate unsigned transaction (offline environment)
npx ts-node starknet/upgrade-contract.ts \
  --env mainnet \
  --contractConfigName AxelarGateway \
  --classHash 0xNewClassHash... \
  --offline \
  --nonce 6 \
  --accountAddress 0x... \
  --l1GasMaxAmount 30000 \
  --l1GasMaxPricePerUnit 100000000000 \
  --l2GasMaxAmount 500000 \
  --l2GasMaxPricePerUnit 1000000000
```

## 🔐 Offline Signing Workflow (Mainnet)

For secure mainnet deployments, follow this complete offline signing workflow:

### Step 1: Gas Estimation (Online Environment)
```bash
# Estimate gas for the transaction
npx ts-node starknet/deploy-contract.ts \
  --env mainnet \
  --contractConfigName AxelarGateway \
  --constructorCalldata '["0x1234"]' \
  --salt 0x123 \
  --estimate \
  --privateKey 0x... \
  --accountAddress 0x...
```

The output will show CLI arguments to copy:
```
--l1GasMaxAmount 50000 --l1GasMaxPricePerUnit 100000000000 --l2GasMaxAmount 1000000 --l2GasMaxPricePerUnit 1000000000
```

### Step 2: Generate Unsigned Transaction (Offline Environment)
Transfer the estimated values to your offline environment and generate the unsigned transaction:

```bash
npx ts-node starknet/deploy-contract.ts \
  --env mainnet \
  --contractConfigName AxelarGateway \
  --constructorCalldata '["0x1234"]' \
  --salt 0x123 \
  --offline \
  --nonce 5 \
  --accountAddress 0x... \
  --l1GasMaxAmount 50000 \
  --l1GasMaxPricePerUnit 100000000000 \
  --l2GasMaxAmount 1000000 \
  --l2GasMaxPricePerUnit 1000000000
```

This creates a file like: `starknet-offline-txs/deploy_AxelarGateway_starknet_2025-06-12T10-30-45-123Z.json`

### Step 3: Sign Transaction with Ledger (Offline Environment)
```bash
# Install Ledger dependencies first (if not already installed)
npm install @ledgerhq/hw-transport-node-hid @ledgerhq/hw-app-starknet

# Sign the transaction
npx ts-node starknet/sign-transaction.ts \
  starknet-offline-txs/deploy_AxelarGateway_starknet_2025-06-12T10-30-45-123Z.json \
  --ledger-path "m/44'/9004'/0'/0/0" \
  --env mainnet
```

**For Multisig Accounts:** Each signer repeats this step independently on their own offline device.

### Step 4: Combine Signatures (For Multisig Only)
If using multisig accounts, combine all signatures:

```bash
npx ts-node starknet/combine-signatures.ts \
  starknet-offline-txs/deploy_AxelarGateway_starknet_2025-06-12T10-30-45-123Z.json \
  starknet-offline-txs/deploy_AxelarGateway_starknet_2025-06-12T10-30-45-123Z_signed.json \
  starknet-offline-txs/deploy_AxelarGateway_starknet_another_signer_signed.json \
  --signers 0x123... 0x456... \
  --output combined_transaction.json
```

### Step 5: Broadcast Transaction (Online Environment)
Transfer the signed transaction to an online environment and broadcast:

```bash
# For single-signature accounts
npx ts-node starknet/broadcast-transaction.ts \
  starknet-offline-txs/deploy_AxelarGateway_starknet_2025-06-12T10-30-45-123Z_signed.json \
  --env mainnet \
  --contract-config-name AxelarGateway

# For multisig accounts
npx ts-node starknet/broadcast-transaction.ts \
  combined_transaction.json \
  --env mainnet \
  --contract-config-name AxelarGateway
```

## 📋 Contract Configuration

Contracts are managed through configuration names stored in the chain config. Each contract entry contains:
- `classHash`: The declared class hash
- `address`: The deployed contract address (after deployment)
- `deploymentTransactionHash`: Transaction hash of deployment
- `declarationTransactionHash`: Transaction hash of declaration
- Other metadata (salt, deployer, timestamps)

## 🛠️ CLI Options Reference

**Base Options (available on all scripts):**
- `-e, --env`: Environment (testnet, mainnet, devnet, stagenet)
- `-y, --yes`: Skip confirmation prompts

**Starknet-Specific Options:**
- `--privateKey`: Private key (testnet only, not required for offline)
- `--accountAddress`: Account address
- `--offline`: Generate unsigned transaction
- `--estimate`: Estimate gas and display CLI arguments to copy
- `--nonce`: Account nonce (required for offline)
- `--outputDir`: Output directory for offline files

**Declare-Specific Options:**
- `--contractConfigName`: Name to store in config
- `--contractPath`: Path to contract JSON file

**Deploy-Specific Options:**
- `--contractConfigName`: Contract configuration name to use
- `--constructorCalldata`: Constructor arguments as JSON array
- `--salt`: Salt for deterministic deployment

**Upgrade-Specific Options:**
- `--contractConfigName`: Contract configuration to upgrade
- `--classHash`: New class hash for upgrade
- `--contractAddress`: Contract address (optional if in config)

**Offline Transaction Gas Options:**
- `--l1GasMaxAmount`: Maximum L1 gas amount
- `--l1GasMaxPricePerUnit`: Maximum L1 gas price per unit
- `--l2GasMaxAmount`: Maximum L2 gas amount  
- `--l2GasMaxPricePerUnit`: Maximum L2 gas price per unit
- `--l1DataMaxAmount`: Maximum L1 data amount
- `--l1DataMaxPricePerUnit`: Maximum L1 data price per unit

**Offline Signing Script Options:**

*sign-transaction.ts:*
- `--ledger-path`: Ledger derivation path (default: "m/44'/9004'/0'/0/0")
- `--env`: Environment for chain ID detection

*combine-signatures.ts:*
- `--signers`: Array of signer addresses corresponding to signature files
- `--output`: Output file for combined transaction

*broadcast-transaction.ts:*
- `--env`: Environment configuration
- `--skip-wait`: Skip waiting for transaction confirmation
- `--contract-config-name`: Contract config name (for deployment transactions)

## 📚 Documentation

### Contract-Specific Guides
- **[Gateway Operations](./docs/gateway.md)** - Cross-chain messaging and gateway management

### Workflow Guides
- **[Offline Signing](./docs/OFFLINE-SIGNING.md)** - Complete guide for mainnet offline workflow
- **[Key Management](./key-management.md)** - Security guidelines and key management

## 🔍 Troubleshooting

### Common Issues

**"Class hash not found in config"**
- Solution: Ensure you've declared the contract first using `declare-contract.ts`

**"Nonce is required for offline transaction generation"**
- Solution: Add `--nonce <current_nonce>` flag

**"Chain not found in configuration"**
- Solution: Verify chain name in `axelar-chains-config/info/<env>.json`

**"Contract path does not exist"**
- Solution: Verify the path to your contract JSON file is correct

### Debug Mode

Add `--verbose` flag to any command for detailed logging.

## 📚 Additional Resources

- [Starknet Official Documentation](https://docs.starknet.io/)
- [Starknet.js Library](https://starknetjs.com/)
- [Axelar Network Documentation](https://docs.axelar.dev/)

## 🤝 Contributing

When adding new contracts:

1. Prepare contract artifacts (sierra and casm JSON files)
2. Declare contract using `declare-contract.ts`
3. Deploy contract using `deploy-contract.ts`
4. Add contract-specific interaction scripts if needed
5. Test on testnet before mainnet

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.

