# Stellar Token Utils v1.0.0

|                | **Owner**                            |
| -------------- | ------------------------------------ |
| **Created By** | @ahramy (<ahram@interoplabs.io>)     |
| **Deployment** | @ahramy (<ahram@interoplabs.io>)     |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Stagenet**         | Completed             | 2025-06-24 |
| **Testnet**          | Completed             | 2025-06-24 |
| **Mainnet**          | Completed             | 2025-06-24 |

- [Crates](https://crates.io/crates/stellar-token-utils/1.0.0)
- [Releases](https://github.com/axelarnetwork/axelar-amplifier-stellar/releases/tag/stellar-token-utils-v1.0.0)

## Background

This is a Stellar Token Utils Release.

Token Utils creates deterministic Stellar Asset Contracts for native Stellar assets. It takes an asset's XDR representation and returns a contract address - either existing or newly deployed. This ensures consistent contract addresses for the same asset across deployments.

## Deployment

Create an `.env` file with the following configuration. Set `CHAIN` according to your target network:
- `stellar-2025-q2` for stagenet
- `stellar-2025-q2-2` for testnet
- `stellar` for mainnet

```yaml
# Change `PRIVATE_KEY in `.env` to Stellar
PRIVATE_KEY=<stellar_deployer_key>
ENV=<stagenet|testnet|mainnet>
CHAIN=<stellar-2025-q2|stellar-2025-q2-2|stellar>
```

1. Deploy Stellar Token Utils

```bash
ts-node stellar/deploy-contract deploy TokenUtils --version 1.0.0
```

## Checklist

Test the contract with an asset and confirm that the returned address is as expected. (i.e. USDC)

## Testing Token Utils

You can verify the expected addresses using the token-utils CLI:

```bash
# Stagenet / Testnet
# Expected contract address: CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
ts-node stellar/token-utils create-stellar-asset-contract USDC GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

```bash
# Mainnet
# Expected contract address: CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75
ts-node stellar/token-utils create-stellar-asset-contract USDC GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN
```
