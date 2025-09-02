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

This release deploys the General Message Passing (GMP) infrastructure for Momento chain, enabling cross-chain communication between Momento and other chains in the Axelar network. This is the foundational layer required for the Duetche Bank integration.

## Deployment

Create an `.env` config. Local environment variable `CHAIN` should be set to `momento`.

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAINS=momento
```

An initial chain config needs to be added to `${ENV}.json` file under `momento` key.

Update npm dependencies (including contracts)

```bash
npm ci && npm run build
```

### Chain Configuration

#### Devnet-Amplifier

```json
{
  "momento": {
    "name": "Momento",
    "axelarId": "momento",
    "chainId": [CHAIN_ID],
    "rpc": "[TESTNET_RPC_URL]",
    "tokenSymbol": "[TOKEN_SYMBOL]",
    "confirmations": 1,
    "finality": "finalized",
    "decimals": 18,
    "approxFinalityWaitTime": 5,
    "chainType": "evm",
    "explorer": {
      "name": "Momento Explorer",
      "url": "[TESTNET_EXPLORER_URL]",
      "api": "[TESTNET_EXPLORER_API]"
    },
    "contracts": {}
  }
}
```

#### Stagenet / Testnet

```json
{
  "momento": {
    "name": "Momento",
    "axelarId": "momento",
    "chainId": [CHAIN_ID],
    "rpc": "[TESTNET_RPC_URL]",
    "tokenSymbol": "[TOKEN_SYMBOL]",
    "confirmations": 6,
    "finality": "finalized",
    "decimals": 18,
    "approxFinalityWaitTime": 15,
    "chainType": "evm",
    "explorer": {
      "name": "Momento Explorer",
      "url": "[TESTNET_EXPLORER_URL]",
      "api": "[TESTNET_EXPLORER_API]"
    },
    "contracts": {}
  }
}
```

#### Mainnet

```json
{
  "momento": {
    "name": "Momento",
    "axelarId": "momento",
    "chainId": [CHAIN_ID],
    "rpc": "[MAINNET_RPC_URL]",
    "tokenSymbol": "[TOKEN_SYMBOL]",
    "confirmations": 12,
    "finality": "finalized",
    "decimals": 18,
    "approxFinalityWaitTime": 30,
    "chainType": "evm",
    "explorer": {
      "name": "Momento Explorer",
      "url": "[MAINNET_EXPLORER_URL]",
      "api": "[MAINNET_EXPLORER_API]"
    },
    "contracts": {}
  }
}
```

### Live Network Testing

Perform [Live network testing](https://github.com/axelarnetwork/axelar-cgp-solidity?tab=readme-ov-file#live-network-testing) to verify that the RPC endpoint is EVM-compatible and the Axelar gateway can be deployed on the external network.

```bash
# Test RPC compatibility
npx hardhat test --network momento --grep "RpcCompatibility"

# Test Gateway deployment
npx hardhat test --network momento --grep "AxelarGateway"
```

### Deployment Commands

#### 1. Deploy Amplifier Gateway

```bash
ts-node evm/deploy-amplifier-gateway.js -e $ENV -n momento
```

For debugging with wallet as signer:
```bash
ts-node evm/deploy-amplifier-gateway.js -e $ENV -n momento --keyID deployer --owner [OWNER_ADDRESS]
```

#### 2. Deploy Gas Service

```bash
ts-node evm/deploy-gas-service.js -e $ENV -n momento
```

#### 3. Deploy Constant Address Deployer (if needed)

```bash
# Deploy Create2 Deployer
ts-node evm/deploy-contract.js -c Create2Deployer -m create -n momento

# Deploy Create3 Deployer
ts-node evm/deploy-contract.js -c Create3Deployer -m create2 -n momento
```

### Contract Verification

After deployment, verify all contracts on the block explorer:

```bash
# Verify Gateway
ts-node evm/verify-contract.js --network momento --contract AxelarGateway

# Verify Gas Service
ts-node evm/verify-contract.js --network momento --contract AxelarGasService

# Verify Auth Module
ts-node evm/verify-contract.js --network momento --contract AxelarAuthWeighted
```

## Configuration Updates

### Update Axelar Chains Config

Add Momento to the chains configuration repository:

```bash
# Clone axelar-chains-config
git clone https://github.com/axelarnetwork/axelar-chains-config
cd axelar-chains-config

# Add Momento configuration to info/${ENV}.json
```

### Submit Amplifier Configuration

For Amplifier networks, submit the chain configuration:

```bash
ts-node cosmwasm/submit-proposal \
    amplifier-add-chain momento \
    -t "Add Momento chain to Amplifier" \
    -d "Integration of Momento chain for Duetche Bank" \
    --deposit 100000000uaxl
```

## Checklist

### Pre-Deployment
- [ ] Chain ID confirmed with Duetche Bank team
- [ ] RPC endpoints tested and responsive
- [ ] Explorer API available and functional
- [ ] Deployer wallet funded with native tokens
- [ ] Configuration added to all environment files

### Deployment Verification
- [ ] Gateway deployed and verified
- [ ] Gas Service deployed and verified
- [ ] Constant address deployers available
- [ ] All contracts verified on explorer
- [ ] Configuration saved to repository

### Integration Testing
- [ ] Send test message from Momento to Ethereum
- [ ] Receive test message on Momento from Ethereum
- [ ] Gas payment successful
- [ ] Message execution confirmed

### Post-Deployment
- [ ] Contract addresses documented
- [ ] Monitoring alerts configured
- [ ] Documentation updated
- [ ] Team notified of deployment status

## Deployed Addresses

| Contract | Devnet | Testnet | Mainnet |
|----------|--------|---------|---------|
| Gateway  | 0x...  | 0x...   | 0x...   |
| Gas Service | 0x... | 0x... | 0x...   |
| Create2 Deployer | 0x... | 0x... | 0x... |
| Create3 Deployer | 0x... | 0x... | 0x... |

## Support

For technical support or questions:
- Duetche Bank Team: [technical-team@duetchebank.com]
- Axelar Support: [support@axelar.network]
- Emergency Contact: [24/7 contact information]

## Notes

- Ensure all transactions are confirmed before proceeding to the next step
- Keep private keys secure and never commit them to the repository
- Document any deviations from the standard deployment process
- Report any issues immediately to both teams