## Monad Transceiver Deployment v1.1.0

|                | **Owner**                                                                     |
|----------------|-------------------------------------------------------------------------------|
| **Created By** | @kulikthebird <tomasz@interoplabs.io>, @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>                                        |

| **Network**          | **Deployment Status** | **Date** |
|----------------------|-----------------------|----------|
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

## Background

- This is the Monad Axelar/Wormhole Transceiver release. This release deploys AxelarTransceive & ERC1967Proxy from example-wormhole-axelar-wsteth [repo](https://github.com/wormhole-foundation/example-wormhole-axelar-wsteth).

## Deployment

Ensure that [Monad GMP](../evm/2025-05-Monad-GMP-v6.0.4.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAIN=xyz
```

| `Network`            | `deployer address`                           |
|----------------------|----------------------------------------------|
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

## AxelarTransceiver and ERC1967 Proxy Deployment

### Prerequisites

1. Install Foundry: https://github.com/foundry-rs/foundry
2. Clone Transceiver repo: `git clone https://github.com/wormhole-foundation/example-wormhole-axelar-wsteth.git`
3. `cd` into `example-wormhole-axelar-wsteth` 
4. Get address of already deployed transceiverStructs library and set value:
`TRANSCEIVER_STRUCTS_ADDRESS=0x..`
5. Run: `forge build --out out --libraries "lib/example-native-token-transfers/evm/src/libraries/TransceiverStructs.sol:TransceiverStructs:$TRANSCEIVER_STRUCTS_ADDRESS"`

1. Set Environment Variables

```bash
ENV=xyz
CHAIN=xyz
```

2. Set address of deployed `gmpManager` & `TransceiverStructs` to the `AxelarTransceiver` section in your chain config:

```json
"AxelarTransceiver": {
  "gmpManager": "$GMP_MANAGER_ADDRESS",
  "TransceiverStructs": "$TRANSCEIVER_STRUCTS_ADDRESS"
}
```

3. Deploy AxelarTransceiver contract

- Notes:
    - We use `create` method to deploy, because AxelarTransceiver deployer will be used to initialize the contract
    - `--artifactPath` is required for AxelarTransceiver deployment and should point to example-wormhole-axelar-wsteth/out/
    - The `gmpManager` address is automatically read from the chain config (`AxelarTransceiver.gmpManager`)
    - Library Linking: Pre-linked artifacts are generated and required libraries are already linked

```bash
ts-node evm/deploy-contract.js \
  -c AxelarTransceiver \
  -m create \
  --artifactPath path/to/example-wormhole-axelar-wsteth/out/
```

4. Deploy ERC1967Proxy 

- Notes:
    - We use `create` method to deploy for ERC1967Proxy of AxelarTransceiver, to maintain consistency
    - `--artifactPath` is required for ERC1967Proxy deployment and should point to example-wormhole-axelar-wsteth/out/
    - `--forContract` is required flag & should have value `AxelarTransceiver`

```bash
ts-node evm/deploy-contract.js \
  -c ERC1967Proxy \
  -m create \
  --artifactPath path/to/example-wormhole-axelar-wsteth/out/ \
  --forContract AxelarTransceiver
```

5. Initialize AxelarTransceiver 

- Initialize step will set
    - GmpManger's owner as `owner` of AxelarTransceiver contract
    - Deployer as `pauser` of AxelarTransceiver contract

```bash
ts-node evm/axelar-transceiver.ts initialize --artifactPath path/to/example-wormhole-axelar-wsteth/out/
```

## Registration (Wormhole's End)

- Note: These commands can only be called by `owner` of gmpManager contract.

1. Set Environment Variables

```bash
ENV=xyz
CHAIN=xyz
PRIVATE_KEY=0x.. # Owner of gmpManager contract
THRESHOLD_VALUE=2 # Unconfirmed
```

| `NETWORK`   | `CHAIN`  | `WORMHOLE_CHAIN_ID` | `AXELAR_CHAIN_NAME` | `TRANSCEIVER_ADDRESS` |
|-------------|----------|---------------------|---------------------|-----------------------|
| **Testnet** | Ethereum |                     | `ethereum-sepolia`  |                       |
|             | Monad    |                     | `monad`             |                       |
| **Mainnet** | Ethereum |                     | `ethereum`          |                       |
|             | Monad    |                     | `monad`             |                       |

2. Set Axelar Chain ID

```bash
ts-node evm/axelar-transceiver.ts set-axelar-chain-id $WORMHOLE_CHAIN_ID $AXELAR_CHAIN_NAME $TRANSCEIVER_ADDRESS --artifactPath path/to/example-wormhole-axelar-wsteth/out/
```

3. Set AxelarTransceiver contract on gmpManger

```bash
GmpMangerProxy.setTransceiver(address AxelarTransceiverProxy)
```

4. Update threshold value

Note: 
- Threshold is not auto-increased to avoid breaking in-flight message redemption.
- The owner must manually update it after all chains adopt the AxelarTransceiver

```bash
GmpMangerPxoxy.setThreshold(uint8 $THRESHOLD_VALUE)
```

### Verify Contracts

Manually verify both contracts i.e. AxelarTransceiver & ERC1967Proxy, foundry artifacts can be found on example-wormhole-axelar-wsteth repo.

## Checklist

<!-- TODO: need tests -->
