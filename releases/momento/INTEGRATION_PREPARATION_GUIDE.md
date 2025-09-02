# Momento Chain Integration - Release Documentation Preparation Guide

## Overview
This document outlines the comprehensive steps required to prepare release documentation for integrating Momento chain with Duetche Bank's EVM infrastructure through Axelar's cross-chain protocol.

## Table of Contents
1. [Pre-Integration Requirements](#pre-integration-requirements)
2. [Phase 1: GMP (General Message Passing) Release](#phase-1-gmp-general-message-passing-release)
3. [Phase 2: ITS (Interchain Token Service) Release](#phase-2-its-interchain-token-service-release)
4. [Phase 3: Amplifier Configuration](#phase-3-amplifier-configuration)
5. [Phase 4: Testing & Verification](#phase-4-testing--verification)
6. [Phase 5: Documentation & Governance](#phase-5-documentation--governance)

---

## Pre-Integration Requirements

### 1. Chain Information Gathering
- [ ] **Chain Name**: Momento
- [ ] **Chain Type**: EVM-compatible
- [ ] **Chain ID**: Obtain from Momento/Duetche Bank team
- [ ] **RPC Endpoints**: 
  - Testnet RPC URL
  - Mainnet RPC URL
- [ ] **Native Token**:
  - Symbol
  - Decimals (typically 18 for EVM chains)
- [ ] **Block Explorer**:
  - Explorer name
  - Explorer URL
  - Explorer API endpoint (if available)
- [ ] **Network Parameters**:
  - Block confirmation requirements
  - Finality mechanism (finalized/confirmationHeight)
  - Approximate finality wait time

### 2. Access & Permissions
- [ ] Deployer wallet addresses for each environment
- [ ] Private keys secured in `.env` file
- [ ] Access to Momento testnet faucet
- [ ] Communication channel with Duetche Bank technical team

### 3. Environment Setup
```bash
# Clone and setup the repository
git clone [repository-url]
cd axelar-contract-deployments
npm ci
npm run build

# Create environment file
cp .example.env .env
# Add PRIVATE_KEY=<deployer_private_key>
```

---

## Phase 1: GMP (General Message Passing) Release

### Document: `/releases/evm/2025-XX-Momento-GMP-v6.X.X.md`

#### 1.1 Create GMP Release Document
Use the template from `/releases/evm/EVM-GMP-Release-Template.md`

```markdown
# Momento GMP v6.X.X

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @[github-username] <email@duetchebank.com> |
| **Deployment** | @[github-username] <email@duetchebank.com> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

- [Release](https://github.com/axelarnetwork/axelar-cgp-solidity/releases/tag/vX.X.X)

## Background
Initial GMP deployment for Momento chain integration with Duetche Bank infrastructure.

## Deployment Steps
[Detailed deployment instructions]
```

#### 1.2 Chain Configuration
Add to `${ENV}.json` (devnet-amplifier.json, testnet.json, mainnet.json):

```json
{
  "momento": {
    "name": "Momento",
    "axelarId": "momento",
    "chainId": [CHAIN_ID],
    "rpc": "[RPC_URL]",
    "tokenSymbol": "[TOKEN_SYMBOL]",
    "confirmations": 6,
    "finality": "finalized",
    "decimals": 18,
    "approxFinalityWaitTime": 15,
    "chainType": "evm",
    "explorer": {
      "name": "Momento Explorer",
      "url": "[EXPLORER_URL]",
      "api": "[EXPLORER_API_URL]"
    },
    "contracts": {}
  }
}
```

#### 1.3 Deployment Commands
```bash
# Set environment variables
export ENV=devnet-amplifier  # or testnet, mainnet
export CHAIN=momento

# Test RPC compatibility
ts-node evm/test-rpc.js --network momento

# Deploy Amplifier Gateway
ts-node evm/deploy-amplifier-gateway.js -e $ENV -n momento

# Deploy Gas Service
ts-node evm/deploy-gas-service.js -e $ENV -n momento
```

---

## Phase 2: ITS (Interchain Token Service) Release

### Document: `/releases/evm/2025-XX-Momento-ITS-v2.X.X.md`

#### 2.1 Create ITS Release Document
Use the template from `/releases/evm/EVM-ITS-Release-Template.md`

```markdown
# Momento ITS v2.X.X

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @[github-username] <email@duetchebank.com> |
| **Deployment** | @[github-username] <email@duetchebank.com> |

[Status table and deployment instructions]
```

#### 2.2 ITS Deployment
```bash
# Devnet Amplifier
ts-node evm/deploy-its.js -s "v2.X.X devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'

# Testnet/Mainnet
ts-node evm/deploy-its.js -s "v2.X.X" -m create2 --proxySalt 'v1.0.0'
```

#### 2.3 Trust Chain Setup
```bash
# Set Momento as trusted chain on remote ITS contracts
ts-node evm/its set-trusted-chains momento -n all

# Register on ITS Hub
ts-node cosmwasm/submit-proposal \
    its-hub-register-chains momento \
    -t "Register Momento on ITS Hub" \
    -d "Register Momento chain for Duetche Bank integration" \
    --deposit 100000000uaxl
```

---

## Phase 3: Amplifier Configuration

### Document: `/releases/ampd/2025-XX-momento-ampd-config.md`

#### 3.1 Ampd Handler Configuration
Add to ampd configuration files:

```toml
[[handlers]]
chain_name="momento"
chain_rpc_url="[MOMENTO_RPC_URL]"
cosmwasm_contract="[VERIFIER_CONTRACT_ADDRESS]"
type="EvmMsgVerifier"

[[handlers]]
chain_name="momento"
cosmwasm_contract="[PROVER_CONTRACT_ADDRESS]"
type="EvmMultisigProver"
```

#### 3.2 Verifier Deployment
```bash
# Deploy verifier contract on Axelar
ts-node cosmwasm/deploy-contract.js \
    --contract EvmVerifier \
    --chain momento \
    --version v1.0.0
```

---

## Phase 4: Testing & Verification

### 4.1 Contract Verification
```bash
# Verify all deployed contracts on block explorer
ts-node evm/verify-contract.js \
    --network momento \
    --contract Gateway

ts-node evm/verify-contract.js \
    --network momento \
    --contract InterchainTokenService
```

### 4.2 Integration Tests
Create test document: `/releases/momento/TEST_RESULTS.md`

#### Test Checklist:
- [ ] **Gateway Tests**
  - [ ] Deploy test token
  - [ ] Send message from Momento to another chain
  - [ ] Receive message on Momento from another chain
  - [ ] Verify message execution

- [ ] **ITS Tests**
  - [ ] Deploy interchain token
  - [ ] Transfer token from Momento
  - [ ] Receive token on Momento
  - [ ] Verify balance updates

- [ ] **Gas Service Tests**
  - [ ] Pay gas on source chain
  - [ ] Verify gas payment collection
  - [ ] Test refund mechanism

### 4.3 End-to-End Test Commands
```bash
# Test GMP message passing
ts-node evm/test-gmp.js \
    --source ethereum \
    --destination momento \
    --message "Hello from Ethereum"

# Test ITS token transfer
ts-node evm/test-its.js \
    --source momento \
    --destination polygon \
    --token USDC \
    --amount 100
```

---

## Phase 5: Documentation & Governance

### 5.1 Technical Documentation
Create `/docs/momento-integration.md`:
- Architecture overview
- Contract addresses (testnet/mainnet)
- Integration examples
- Troubleshooting guide

### 5.2 Governance Proposal
For mainnet deployment, prepare governance proposal:

```markdown
# Proposal: Add Momento Chain to Axelar Network

## Summary
Integration of Momento chain (Duetche Bank) into Axelar's cross-chain infrastructure.

## Details
- Chain ID: [CHAIN_ID]
- Native Token: [TOKEN_SYMBOL]
- Use Case: [Banking/Financial applications]

## Technical Specifications
[Include contract addresses and configuration]

## Security Considerations
[Audit reports, security measures]
```

### 5.3 Community Communication
- [ ] Create forum post on community.axelar.network
- [ ] Prepare announcement for Discord/Telegram
- [ ] Update documentation on docs.axelar.dev

---

## Release Checklist

### Pre-Deployment
- [ ] Chain configuration added to all environment files
- [ ] RPC endpoints tested and verified
- [ ] Deployer wallets funded
- [ ] Release branch created

### Deployment Tracking
| Component | Devnet | Testnet | Mainnet |
|-----------|--------|---------|---------|
| Gateway   | [ ]    | [ ]     | [ ]     |
| Gas Service | [ ]  | [ ]     | [ ]     |
| ITS       | [ ]    | [ ]     | [ ]     |
| Verifier  | [ ]    | [ ]     | [ ]     |

### Post-Deployment
- [ ] Contracts verified on explorer
- [ ] Integration tests passed
- [ ] Documentation updated
- [ ] Monitoring configured
- [ ] Incident response plan documented

---

## File Structure
```
releases/
├── momento/
│   ├── INTEGRATION_PREPARATION_GUIDE.md (this file)
│   ├── 2025-XX-GMP-v6.X.X.md
│   ├── 2025-XX-ITS-v2.X.X.md
│   ├── TEST_RESULTS.md
│   └── DEPLOYMENT_ADDRESSES.md
├── ampd/
│   └── 2025-XX-momento-config.md
└── cosmwasm/
    └── 2025-XX-momento-verifier.md
```

---

## Support & Resources

### Internal Resources
- Axelar Documentation: https://docs.axelar.dev
- GitHub Repository: https://github.com/axelarnetwork/axelar-contract-deployments
- Release Templates: `/releases/evm/`

### Contact Points
- Technical Support: [technical-team@duetchebank.com]
- Axelar Team: [support@axelar.network]
- Emergency Contact: [Include 24/7 contact]

---

## Version History
| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0   | 2025-XX-XX | [Name] | Initial document |

---

*This document should be reviewed and updated regularly as the integration progresses.*