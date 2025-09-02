# Momento ITS v2.X.X

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

- [Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/vX.X.X)

## Background

This release deploys the Interchain Token Service (ITS) on Momento chain, enabling seamless token transfers between Momento and other chains in the Axelar network. This is a critical component for the Duetche Bank integration, allowing for cross-chain asset transfers and custom token deployments.

## Prerequisites

Ensure that [Momento GMP](./2025-XX-GMP-v6.X.X.md) is deployed first.

## Deployment

```bash
# Clone latest main and update deps
npm ci && npm run build
```

Create an `.env` config

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAIN=momento
```

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Devnet Amplifier

```bash
# Deploy ITS with CREATE2 for deterministic addresses
ts-node evm/deploy-its.js -s "v2.X.X devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet

```bash
# Deploy ITS
ts-node evm/deploy-its.js -s "v2.X.X" -m create2 --proxySalt 'v1.0.0'
```

### Mainnet

```bash
# Deploy ITS with production salt
ts-node evm/deploy-its.js -s "v2.X.X" -m create2 --proxySalt 'v1.0.0'
```

## Post-Deployment Configuration

### 1. Register Momento on ITS Hub

ITS hub contract configuration in `$ENV.json` must include the following attributes:

```json
{
  "axelar": {
    "contracts": {
      "InterchainTokenService": {
        "momento": {
          "maxUintBits": 256,
          "maxDecimalsWhenTruncating": 255
        }
      }
    }
  }
}
```

Submit the registration proposal:

```bash
# For devnet-amplifier (with runAs account)
ts-node cosmwasm/submit-proposal \
    its-hub-register-chains momento \
    -t "Register Momento on ITS Hub" \
    -d "Register Momento chain for Duetche Bank integration" \
    --deposit 100000000uaxl \
    --runAs [RUN_AS_ACCOUNT]

# For stagenet/testnet/mainnet (no runAs)
ts-node cosmwasm/submit-proposal \
    its-hub-register-chains momento \
    -t "Register Momento on ITS Hub" \
    -d "Register Momento chain for Duetche Bank integration" \
    --deposit 100000000uaxl
```

### 2. Set Trusted Chains

#### On Momento ITS

Set all existing chains as trusted on Momento:

```bash
# Add all trusted chains to Momento ITS
ts-node evm/its set-trusted-chains all -n momento
```

Or add specific chains:

```bash
# Add specific trusted chains
ts-node evm/its set-trusted-chains ethereum polygon avalanche -n momento
```

#### On Remote ITS Contracts

Set Momento as trusted chain on all other EVM chains:

```bash
# Set Momento as trusted on all EVM chains
ts-node evm/its set-trusted-chains momento -n all
```

For non-EVM chains, set Momento as trusted individually:

```bash
# Example for Stellar
PRIVATE_KEY=<stellar_deployer_key>
ts-node stellar/its add-trusted-chains momento

# Example for Sui
PRIVATE_KEY=<sui_deployer_key>
ts-node sui/its.js set-trusted-chain momento
```

### 3. Deploy Supporting Contracts

#### Token Manager Factory (if needed)

```bash
ts-node evm/deploy-contract.js \
    -c TokenManagerFactory \
    -n momento \
    -m create2 \
    -s "v1.0.0"
```

#### Example Token (for testing)

```bash
ts-node evm/deploy-contract.js \
    -c ExampleToken \
    -n momento \
    --name "Test USDC" \
    --symbol "USDC" \
    --decimals 6
```

## Contract Verification

Verify all deployed ITS contracts on the block explorer:

```bash
# Verify ITS Proxy
ts-node evm/verify-contract.js \
    --network momento \
    --contract InterchainTokenService

# Verify ITS Implementation
ts-node evm/verify-contract.js \
    --network momento \
    --contract InterchainTokenServiceImplementation

# Verify Token Manager
ts-node evm/verify-contract.js \
    --network momento \
    --contract TokenManager

# Verify Token Handler
ts-node evm/verify-contract.js \
    --network momento \
    --contract TokenHandler
```

## Testing & Validation

### 1. Deploy Interchain Token

```bash
# Deploy a test interchain token
ts-node evm/its deploy-interchain-token \
    --name "Test Token" \
    --symbol "TEST" \
    --decimals 18 \
    --initialSupply 1000000 \
    --network momento
```

### 2. Token Transfer Tests

#### Momento to Ethereum

```bash
# Send tokens from Momento to Ethereum
ts-node evm/its transfer \
    --source momento \
    --destination ethereum \
    --token TEST \
    --amount 100 \
    --recipient [RECIPIENT_ADDRESS]
```

#### Ethereum to Momento

```bash
# Send tokens from Ethereum to Momento
ts-node evm/its transfer \
    --source ethereum \
    --destination momento \
    --token USDC \
    --amount 100 \
    --recipient [RECIPIENT_ADDRESS]
```

### 3. Custom Token Linking

For existing tokens on Momento that need to be linked:

```bash
# Link existing token on Momento
ts-node evm/its link-token \
    --network momento \
    --token [TOKEN_ADDRESS] \
    --tokenManager [MANAGER_TYPE] \
    --params [PARAMS]
```

## Checklist

### Pre-Deployment
- [ ] GMP deployed and verified on Momento
- [ ] Deployer wallet funded
- [ ] ITS Hub upgraded to compatible version
- [ ] Environment configuration updated

### Deployment Verification
- [ ] ITS Proxy deployed
- [ ] ITS Implementation deployed
- [ ] Token Manager deployed
- [ ] All contracts verified on explorer

### Configuration
- [ ] Momento registered on ITS Hub
- [ ] Trusted chains configured on Momento
- [ ] Momento set as trusted on remote chains
- [ ] Gas service integration configured

### Testing
- [ ] Test token deployed
- [ ] Outbound transfer successful
- [ ] Inbound transfer successful
- [ ] Token balance updates verified
- [ ] Gas payment processed

### Post-Deployment
- [ ] Contract addresses documented
- [ ] Integration guide updated
- [ ] Monitoring configured
- [ ] Support team notified

## Deployed Addresses

| Contract | Devnet | Testnet | Mainnet |
|----------|--------|---------|---------|
| ITS Proxy | 0x... | 0x... | 0x... |
| ITS Implementation | 0x... | 0x... | 0x... |
| Token Manager | 0x... | 0x... | 0x... |
| Token Handler | 0x... | 0x... | 0x... |
| Token Manager Factory | 0x... | 0x... | 0x... |

## Integration Examples

### Deploy Custom Token

```javascript
const { ethers } = require('ethers');
const ITS = require('./artifacts/IInterchainTokenService.json');

async function deployCustomToken() {
    const provider = new ethers.providers.JsonRpcProvider(RPC_URL);
    const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
    const its = new ethers.Contract(ITS_ADDRESS, ITS.abi, wallet);
    
    // Deploy interchain token
    const salt = ethers.utils.id("CUSTOM_TOKEN_SALT");
    const tx = await its.deployInterchainToken(
        salt,
        "Custom Token",
        "CUSTOM",
        18,
        ethers.utils.parseEther("1000000"),
        wallet.address
    );
    
    await tx.wait();
    console.log("Token deployed:", tx.hash);
}
```

### Transfer Tokens Cross-Chain

```javascript
async function transferTokens() {
    const provider = new ethers.providers.JsonRpcProvider(RPC_URL);
    const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
    const its = new ethers.Contract(ITS_ADDRESS, ITS.abi, wallet);
    
    // Transfer tokens to another chain
    const tokenId = await its.interchainTokenId(wallet.address, salt);
    const amount = ethers.utils.parseEther("100");
    const destinationChain = "ethereum";
    const destinationAddress = "0x...";
    
    const tx = await its.interchainTransfer(
        tokenId,
        destinationChain,
        destinationAddress,
        amount,
        "0x",
        0
    );
    
    await tx.wait();
    console.log("Transfer initiated:", tx.hash);
}
```

## Troubleshooting

### Common Issues

1. **"Chain not trusted" error**
   - Ensure trusted chains are properly configured on both source and destination

2. **"Insufficient gas" error**
   - Verify gas payment on source chain
   - Check gas service configuration

3. **"Token not found" error**
   - Confirm token is deployed on both chains
   - Verify token ID matches across chains

### Debug Commands

```bash
# Check ITS configuration
ts-node evm/its info --network momento

# Verify trusted chains
ts-node evm/its get-trusted-chains --network momento

# Check token deployment
ts-node evm/its get-token-info --token [TOKEN_ID] --network momento
```

## Support

For technical support or questions:
- Duetche Bank Team: [technical-team@duetchebank.com]
- Axelar Support: [support@axelar.network]
- ITS Documentation: https://docs.axelar.dev/dev/send-tokens/interchain-tokens

## Notes

- Always test token transfers on testnet before mainnet deployment
- Monitor gas prices on Momento for optimal transaction fees
- Keep track of token IDs for cross-chain consistency
- Document any custom token configurations or special requirements