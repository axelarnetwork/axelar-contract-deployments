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

- This is the Monad Axelar/Wormhole Transceiver release. Wormhole uses their own fork of OpenZeppelin contract, thus we are using external repo to deploy contracts.

- This release deploys AxelarTransceiver & ERC1967Proxy contracts from the example-wormhole-axelar-wsteth [repo](https://github.com/wormhole-foundation/example-wormhole-axelar-wsteth).

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

| `Network`   | `deployer address`                           |
|-------------|----------------------------------------------|
| **Testnet** | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet** | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

## AxelarTransceiver and ERC1967 Proxy Deployment

### Prerequisites

1. Install Foundry: https://github.com/foundry-rs/foundry
2. Clone Transceiver repo: `git clone https://github.com/wormhole-foundation/example-wormhole-axelar-wsteth.git`
3. `cd` into `example-wormhole-axelar-wsteth` 

| `NETWORK`   | `CHAIN`  | `TRANSCEIVER_STRUCTS_ADDRESS` | `GMP_MANAGER_ADDRESS` |
|-------------|----------|-------------------------------|-----------------------|
| **Testnet** | Ethereum |                               |                       |
|             | Monad    |                               |                       |
| **Mainnet** | Ethereum |                               |                       |
|             | Monad    |                               |                       |

4. Get address of already deployed transceiverStructs library and set value:
`TRANSCEIVER_STRUCTS_ADDRESS=0x..`

- Note: Deployed `TRANSCEIVER_STRUCTS_ADDRESS` should be confirmed with Wormhole for each chain

5. Run: `forge build --out out --libraries "lib/example-native-token-transfers/evm/src/libraries/TransceiverStructs.sol:TransceiverStructs:$TRANSCEIVER_STRUCTS_ADDRESS"`

### Deployment

1. Set Environment Variables

```bash
ENV=xyz
CHAIN=xyz
```

2. Set address of deployed `gmpManager` & `TransceiverStructs` to the `config.chains.$CHAIN.contracts.AxelarTransceiver` section in your chain config:

```json
"AxelarTransceiver": {
  "gmpManager": "$GMP_MANAGER_ADDRESS",
  "TransceiverStructs": "$TRANSCEIVER_STRUCTS_ADDRESS"
}
```

3. Deploy AxelarTransceiver contract

- Notes:
    - We use `create` method to deploy, because AxelarTransceiver deployer will be used to initialize the contract
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
    - GmpManager's owner as `owner` of AxelarTransceiver contract
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

3. Set AxelarTransceiver contract on GmpManager

```bash
GmpManagerProxy.setTransceiver(address AxelarTransceiverProxy)
```

4. Update threshold value

Note: 
- Threshold is not auto-increased to avoid breaking in-flight message redemption.
- The owner must manually update it after all chains adopt the AxelarTransceiver

```bash
GmpManagerProxy.setThreshold(uint8 $THRESHOLD_VALUE)
```

### Verify Contracts

Manually verify both contracts i.e. AxelarTransceiver & ERC1967Proxy, foundry artifacts can be found on example-wormhole-axelar-wsteth repo.

## Checklist

### Ethereum -> Monad 

1. Initiate a transaction from ethereum to monad on [Bridge](https://monadbridge.com/)

2. Open up tx on ethereum explorer like [this](https://sepolia.etherscan.io/tx/0x417d5fadffecc197921ddf6893bdc0a3cc1b74059d293fdacc49cfefa830129c)

3. Go to logs section and check for `SendTransceiverMessage` event, you'll find two transactions, with Transceiver address for both Wormhole and Axelar.

4. Crosscheck deployed `AxelarTransceiver` address, AxelarTransceiver should emit `RelayingInfo` event 

### Monad -> Ethereum

1. Initiate a transaction from monad to ethereum on [Bridge](https://monadbridge.com/)

2. Open up transaction on monad explorer

3. Go to logs section and check for `SendTransceiverMessage` event, you'll find two transactions, with Transceiver address for both Wormhole and Axelar.

4. Crosscheck deployed `AxelarTransceiver` address, AxelarTransceiver should emit `RelayingInfo` event 
