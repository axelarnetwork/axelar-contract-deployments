## XRPL EVM Sidechain ITS v2.1.0

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io> |
| **Deployment** |                                    |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
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

Ensure that [Monad GMP](../evm/2025-03-Monad-GMP-v1.0.0.md) is deployed first.

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
| -------------------- | -------------------------------------------- |
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

### Set Environment Variables

1. Set paths to artifacts in `.env` in this directory:

```yaml
TRANSCEIVER_IMPLEMENTATION=<path/to/example-wormhole-axelar-wsteth/out/AxelarTransceiver.sol/AxelarTransceiver.json>
TRANSCEIVER_PROXY=<path/to/example-wormhole-axelar-wsteth/out/ERC1967Proxy.sol/ERC1967Proxy.json>
TRANSCEIVER_STRUCTS=<path/to/example-wormhole-axelar-wsteth/out/TransceiverStructs.sol/TransceiverStructs.json>
```

2. Set [operator address](../evm/2025-03-Monad-GMP-v1.0.0.md)

```yaml
OPERATOR_ADDRESS=<operator address from ../evm/2025-03-Monad-GMP-v1.0.0.md>
```

3. Set Proxy Salt

```yaml
TODO
```

### Devnet Amplifier

<!-- TODO: Add options -->

```bash
`node evm/deploy-transceiver`
```

### Stagenet / Testnet / Mainnet

<!-- TODO: Add options -->

```bash
`node evm/deploy-transceiver`
```

### Verify Contracts

<!-- TODO -->

## Set Axelar Chain ID

<!-- TODO: need script for this -->

## Registration

<!-- TODO: what is needed here? -->

## Checklist

<!-- TODO: need tests -->
