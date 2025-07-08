## Monad Transceiver Deployment v1.1.0

|                | **Owner**                                                                  |
|----------------|----------------------------------------------------------------------------|
| **Created By** | @AttissNgo <attiss@interoplabs.io>, @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>                                     |

| **Network**          | **Deployment Status** | **Date** |
|----------------------|-----------------------|----------|
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

<!-- TODO -->
<!-- [Release]() -->

## Background

- This is the Monad Axelar/Wormhole Transceiver release.
    <!-- TODO: provide context for partnership, differences from ITS -->

## Deployment

Ensure that [Monad GMP](../evm/2025-05-Monad-GMP-v6.0.4.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. `CHAIN` should be set to `monad`.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAIN=monad
```

| Network              | `deployer address`                           |
|----------------------|----------------------------------------------|
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Create Transceiver Artifacts

Prerequisite: install Foundry (TODO: provide link)

<!-- or use Hardhat?? -->

1. Clone Transceiver repo: https://github.com/wormhole-foundation/example-wormhole-axelar-wsteth
2. `cd` into `example-wormhole-axelar-wsteth` and run `forge build` to create artifacts

- contracts will be compiled and ABIs generated in `out/` directory

### AxelarTrnasceiver Deployments

1. Set Environment Variables

```bash
ENV=xyz
CHAIN=xyz
```

1. Set paths to artifacts in `.env` in this directory:

```bash
TRANSCEIVER_STRUCTS_ADDRESS=0x..
GMP_MANAGER_ADDRESS=0x..
OPERATORS_ADDRESS=Ox..
PROXY_SALT="AXELAR_TRANSCEIVER v1.1.0"
THRESHOLD_VALUE=2
```

2. Deploy AxelarTransceiver contract

```bash
ts-node evm/deploy-contract.js -c AxelarTransceiver --gmpManager $GMP_MANAGER_ADDRESS -m create2 --libraries '{"TransceiverStructs":"$TRANSCEIVER_STRUCTS_ADDRESS"}'
```

3. Deploy ERC1967Proxy 

```bash
ts-node evm/deploy-contract.js -c ERC1967Proxy --forContract AxelarTransceiver
```

4. Initialize AxelarTransceiver 

Initialize step will set
- GmpManger's owner as `owner` of AxelarTransceiver contract
- Deployer as `pauser` of AxelarTransceiver contract

```bash
ts-node evm/axelar-transceiver.ts --initialize
```

5. Set Axelar Chain ID

```bash
ts-node evm/axelar-transceiver.ts --setAxelarChainId <id> <chain>
```

6. Transfer pauser role 

```bash
ts-node evm/axelar-transceiver.ts --pauserAddress $OPERATORS_ADDRESS
```

## Registration (Wormhole's End)

1. Set AxelarTransceiver contract on gmpManger

This can only be called by owner of gmpManager.

```bash
GmpMangerProxy.setTransceiver(address AxelarTransceiverProxy)
```

2. Update threshold value

This can only be called by owner of gmpManager.

Note: 
- Threshold is not auto-increased to avoid breaking in-flight message redemption.
- The owner must manually update it after all chains adopt the AxelarTransceiver

```bash
GmpMangerPxoxy.setThreshold(uint8 $THRESHOLD_VALUE)
```

### Verify Contracts

<!-- TODO: what is needed here? -->

## Checklist

<!-- TODO: need tests -->
