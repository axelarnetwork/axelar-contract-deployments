## Monad Transceiver Deployment v2.0.0

|                | **Owner**                                                                     |
|----------------|-------------------------------------------------------------------------------|
| **Created By** | @kulikthebird <tomasz@interoplabs.io>, @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>                                        |

| **Network**          | **Deployment Status** | **Date**   |
|----------------------|-----------------------|------------|
| **Devnet Amplifier** | -                     | TBD        |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | In Progress           | 2025-07-25 |
| **Mainnet**          | -                     | TBD        |

## Background

- This is the Monad Axelar/Wormhole Transceiver release. Wormhole uses their own fork of OpenZeppelin contract, thus we are using external repo to deploy contracts.

- This release deploys `AxelarTransceiver` used as name `MonadAxelarTransceiver` & `ERC1967Proxy` contracts from the example-wormhole-axelar-ntt-transceiver [repo](https://github.com/wormholelabs-xyz/example-wormhole-axelar-ntt-transceiver).

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

| `Network`   | `deployer address`                           | `ITS_OWNER`                                  |
|-------------|----------------------------------------------|----------------------------------------------|
| **Testnet** | `0x156372Cb2F8939d9705fdaa6C70e25825Ea9CAaF` | `0x49845e5d9985d8dc941462293ed38EEfF18B0eAE` |
| **Mainnet** | `0xf21b824434d93F659aCe9dA3ED3F66480a94E2Fe` | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

## MonadAxelarTransceiver and ERC1967 Proxy Deployment

### Prerequisites

1. Install Foundry: https://github.com/foundry-rs/foundry
2. Clone Transceiver repo: `git clone https://github.com/wormholelabs-xyz/example-wormhole-axelar-ntt-transceiver.git`
3. `cd` into `example-wormhole-axelar-ntt-transceiver` 

| `NETWORK`   | `CHAIN`  | `TRANSCEIVER_STRUCTS_ADDRESS`                | `GMP_MANAGER_ADDRESS`                        |
|-------------|----------|----------------------------------------------|----------------------------------------------|
| **Testnet** | Ethereum | `0x19aA201504dAF1FFBFd7ae6959225996fe84fdc6` | `0xdaee3a6b4196e3e46015b364f1dae54ceae74a91` |
|             | Monad    | `0x19aA201504dAF1FFBFd7ae6959225996fe84fdc6` | `0x641a6608e2959c0D7Fe2a5F267DFDA519ED43d98` |
| **Mainnet** | Ethereum |                     -                        | `0xc6793a32761a11e96c97A3D18fC6545ea931F0E9`                                            |
|             | Monad    |                     -                        | `0x92957b3D0CaB3eA7110fEd1ccc4eF564981a59Fc` |

4. Get address of already deployed transceiverStructs library and set value:
`TRANSCEIVER_STRUCTS_ADDRESS=0x..`

- Note: Deployed `TRANSCEIVER_STRUCTS_ADDRESS` should be confirmed with Wormhole for each chain for testnet

5. Built contract artifacts
#### Mainnet
Run: `forge build --out out`

#### Other ENVs
Run: `forge build --out out --libraries "lib/example-native-token-transfers/evm/src/libraries/TransceiverStructs.sol:TransceiverStructs:$TRANSCEIVER_STRUCTS_ADDRESS"`

### Deployment

1. Set Environment Variables

```bash
ENV=xyz
CHAIN=xyz
```

2. Set address of deployed `gmpManager` & `TransceiverStructs` to the `config.chains.$CHAIN.contracts.MonadAxelarTransceiver` section in your chain config:

```json
"MonadAxelarTransceiver": {
  "gmpManager": "$GMP_MANAGER_ADDRESS",
  "TransceiverStructs": "$TRANSCEIVER_STRUCTS_ADDRESS"
}
```

- Note: Omit TransceiverStructs for mainnet

3. Deploy MonadAxelarTransceiver contract

- Notes:
    - We use `create` method to deploy, because MonadAxelarTransceiver deployer will be used to initialize the contract
    - The `gmpManager` address is automatically read from the chain config (`MonadAxelarTransceiver.gmpManager`)
    - Library Linking: Pre-linked artifacts are generated and required libraries are already linked

```bash
ts-node evm/deploy-contract.js \
  -c AxelarTransceiver \
  -m create \
  --artifactPath path/to/example-wormhole-axelar-ntt-transceiver/out/ \
  --transceiverPrefix Monad
```

4. Deploy ERC1967Proxy 

- Notes:
    - We use `create` method to deploy for ERC1967Proxy of MonadAxelarTransceiver, to maintain consistency
    - `--forContract` is required flag & should have value `MonadAxelarTransceiver`

```bash
ts-node evm/deploy-contract.js \
  -c ERC1967Proxy \
  -m create \
  --artifactPath path/to/example-wormhole-axelar-ntt-transceiver/out/ \
  --forContract MonadAxelarTransceiver
```

5. Initialize MonadAxelarTransceiver

- Initialize step will set
    - GmpManager's owner as `owner` of MonadAxelarTransceiver contract
    - Deployer as `pauser` of MonadAxelarTransceiver contract

```bash
ts-node evm/axelar-transceiver.ts initialize --artifactPath path/to/example-wormhole-axelar-ntt-transceiver/out/ --transceiverPrefix Monad
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

| `NETWORK`   | `CHAIN`  | `WORMHOLE_CHAIN_ID` | `AXELAR_CHAIN_NAME` | `TRANSCEIVER_ADDRESS`                        |
|-------------|----------|---------------------|---------------------|----------------------------------------------|
| **Testnet** | Ethereum |                     | `ethereum-sepolia`  | `0x50beAbe4883981624aEa01F737B040d1e3Fe83FB` |
|             | Monad    |                     | `monad`             | `0x50beAbe4883981624aEa01F737B040d1e3Fe83FB` |
| **Mainnet** | Ethereum |                     | `Ethereum`          | `0x50beAbe4883981624aEa01F737B040d1e3Fe83FB` |
|             | Monad    |                     | `monad`             | `0x50beAbe4883981624aEa01F737B040d1e3Fe83FB` |

2. Set Axelar Chain ID

```bash
ts-node evm/axelar-transceiver.ts set-axelar-chain-id $WORMHOLE_CHAIN_ID $AXELAR_CHAIN_NAME $TRANSCEIVER_ADDRESS --artifactPath path/to/example-wormhole-axelar-ntt-transceiver/out/ --transceiverPrefix Monad
```

3. Set MonadAxelarTransceiver contract on GmpManager

```bash
GmpManagerProxy.setTransceiver(address MonadAxelarTransceiverProxy)
```

4. Update threshold value

Note: 
- Threshold is not auto-increased to avoid breaking in-flight message redemption.
- The owner must manually update it after all chains adopt the MonadAxelarTransceiver

```bash
GmpManagerProxy.setThreshold(uint8 $THRESHOLD_VALUE)
```

## Transfer Pauser role to ITS owner after registration

```bash
ts-node evm/axelar-transceiver.ts transfer-pauser $ITS_OWNER --artifactPath path/to/example-wormhole-axelar-ntt-transceiver/out/ --transceiverPrefix Monad
```

### Verify Contracts

Manually verify both contracts i.e. MonadAxelarTransceiver & ERC1967Proxy, foundry artifacts can be found on example-wormhole-axelar-ntt-transceiver repo.

## Checklist

### Ethereum -> Monad

1. Initiate a transaction from ethereum to monad on [Bridge](https://monadbridge.com/)

2. Open up tx on ethereum explorer like [this](https://sepolia.etherscan.io/tx/0x417d5fadffecc197921ddf6893bdc0a3cc1b74059d293fdacc49cfefa830129c)

3. Go to logs section and check for `SendTransceiverMessage` event, you'll find two transactions, with Transceiver address for both Wormhole and Axelar.

4. Crosscheck deployed `MonadAxelarTransceiver` address, MonadAxelarTransceiver should emit `RelayingInfo` event 

### Monad -> Ethereum

1. Initiate a transaction from monad to ethereum on [Bridge](https://monadbridge.com/)

2. Open up transaction on monad explorer

3. Go to logs section and check for `SendTransceiverMessage` event, you'll find two transactions, with Transceiver address for both Wormhole and Axelar.

4. Crosscheck deployed `MonadAxelarTransceiver` address, MonadAxelarTransceiver should emit `RelayingInfo` event 
