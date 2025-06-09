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
CHAIN_NAMES=starknet-sepolia
```

## 🚀 Core Features

### Dual Workflow Support
- **Online Mode**: Direct transaction execution (testnet/devnet)
- **Offline Mode**: Unsigned transaction generation for hardware wallet signing (mainnet)

### Security Model
- **Testnet/Devnet**: Private key-based signing
- **Mainnet**: Mandatory offline workflow with Ledger hardware wallets

### Transaction Types
- **Invoke Transactions**: Contract calls with L1 data availability support
- **Declare Transactions**: Contract class declarations with compiled class hash validation

### Contract Support
- ✅ Contract deployment and upgrades
- ✅ Gateway operations (call contract, approve messages, validate messages)
- ✅ Signer rotation and operatorship management
- 🔄 Additional contracts (Gas Service, Operators, ITS) - *coming soon*

## 📚 Documentation

### Contract-Specific Guides
- **[Contract Declaration](./docs/declare-contract.md)** - Declare contract classes (includes offline workflow)
- **[Contract Deployment](./docs/deploy-contract.md)** - Deploy and upgrade contracts
- **[Gateway Operations](./docs/gateway.md)** - Cross-chain messaging and gateway management

### Workflow Guides
- **[Offline Signing](./docs/OFFLINE-SIGNING.md)** - Complete guide for mainnet offline workflow
- **[Key Management](./key-management.md)** - Security guidelines and key management

## 🏗️ Contract Architecture

### Supported Contracts

| Contract | Status | Description |
|----------|--------|-------------|
| AxelarGateway | ✅ Implemented | Core gateway for cross-chain messaging |
| AxelarGasService | 🔄 Planned | Gas payment and refund service |
| AxelarOperators | 🔄 Planned | Operator management contract |
| InterchainTokenService | 🔄 Planned | Cross-chain token transfers |
| Governance | 🔄 Planned | Governance and upgrades |

### Contract Artifacts

Contract artifacts should be placed in the `starknet/artifacts/` directory:

```
starknet/artifacts/
├── AxelarGateway/
│   ├── AxelarGateway.contract_class.json
│   └── AxelarGateway.compiled_contract_class.json
└── AxelarGasService/
    ├── AxelarGasService.contract_class.json
    └── AxelarGasService.compiled_contract_class.json
```

## 🛠️ CLI Options Reference

**Base Options (available on all scripts):**
- `-e, --env`: Environment (testnet, mainnet, devnet, stagenet)
- `-n, --chainNames`: Chain names (comma-separated)
- `-y, --yes`: Skip confirmation prompts

**Starknet-Specific Options:**
- `-p, --privateKey`: Private key (testnet only)
- `-a, --accountAddress`: Account address
- `--offline`: Generate unsigned transaction
- `--nonce`: Account nonce (required for offline)
- `--outputDir`: Output directory for offline files

**Declare-Specific Options:**
- `--compiledClassHash`: Compiled class hash (required for offline declare, generate with `starkli class-hash`)
- `--l1DataMaxAmount`: Maximum L1 data amount (default: 128)
- `--l1DataMaxPricePerUnit`: Maximum L1 data price per unit (default: 10000000000)

## 📋 Quick Command Reference

### Declare Contract (Offline)
```bash
# 1. Generate compiled class hash
starkli class-hash path/to/contract.compiled_contract_class.json

# 2. Generate offline declare transaction
npx ts-node starknet/declare-contract.ts \
  --env testnet \
  --chainNames starknet-sepolia \
  --contractName AxelarGateway \
  --offline \
  --nonce 123 \
  --accountAddress 0x1234... \
  --compiledClassHash 0xabcd...
```

### Declare Contract (Online - Testnet Only)
Use `starkli declare...`

## 🔍 Troubleshooting

### Common Issues

**"Nonce is required for offline transaction generation"**
- Solution: Add `--nonce <current_nonce>` flag

**"Contract artifacts not found"**
- Solution: Ensure artifacts are in `starknet/artifacts/<ContractName>/`

**"Chain not found in configuration"**
- Solution: Verify chain name in `axelar-chains-config/info/<env>.json`

**"Account address required for offline transaction generation"**
- Solution: Add `--accountAddress 0x...` flag

**"Compiled class hash is required for offline declare transaction"**
- Solution: Generate with `starkli class-hash <compiled_contract_class.json>` and use `--compiledClassHash 0x...`

### Debug Mode

Add `--verbose` flag to any command for detailed logging.

## 📚 Additional Resources

- [Starknet Official Documentation](https://docs.starknet.io/)
- [Starknet.js Library](https://starknetjs.com/)
- [Axelar Network Documentation](https://docs.axelar.dev/)

## 🤝 Contributing

When adding new contracts:

1. Add artifact files to `starknet/artifacts/<ContractName>/`
2. Update CLI options in `cli-utils.js` if needed
3. Add contract-specific interaction scripts
4. Create documentation in `docs/<contract>.md`
5. Test on testnet before mainnet

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.

