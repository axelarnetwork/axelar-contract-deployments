# Cosmwasm Multisig v1.2.0

|  | **Owner** |
|-----------|------------|
| **Created By** | @cjcobb23 <cj@interoplabs.io> |
| **Deployment** | TBD |

| **Network** | **Deployment Status** | **Date** |
|-------------|----------------------|----------|
| **Devnet Amplifier** | Deployed | 2025-05-06 |
| **Stagenet** | - | TBD |
| **Testnet** | - | TBD |
| **Mainnet** | - | TBD |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/interchain-token-service-v1.2.1)

## Background

Changes in this release:

1. Use error-stack for better error reporting

## Deployment

- This rollout upgrades the amplifier multisig contract from `v1.1.1` to `v1.2.0`
- There is a no state migration involved

1. Download interchain token service wasm bytecode

```bash
mkdir wasm
wget https://static.axelar.network/releases/cosmwasm/multisig/1.2.0/multisig.wasm --directory-prefix=wasm/
```

2. Download and verify Checksum
```bash
wget https://static.axelar.network/releases/cosmwasm/multisig/1.2.0/checksums.txt
CHECKSUM=$(cat checksums.txt | grep multisig.wasm | awk '{print $1}')
shasum -a 256 wasm/multisig.wasm | grep $CHECKSUM
```

3. Expected output, make sure this matches before proceeding
```
6898517f95da74cbc8c59223cc0f0f6b89e95911c321cf3be87e999935366687  wasm/multisig.wasm
```

4. Upload new ITS Hub contract

| environment | INIT_ADDRESSES    |  RUN_AS_ACCOUNT |
| :-----: | :---: | :---: |
| devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`  | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |
| stagenet | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky`    | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |
| testnet | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7`   | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |
| mainnet | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am`   | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |

```bash
node cosmwasm/submit-proposal.js store -c Multisig -t "Upload Multisig contract v1.2.0" -d "Upload Multisig contract v1.2.0" -r $RUN_AS_ACCOUNT --deposit 2000000000 --instantiateAddresses $INIT_ADDRESSES -a ./wasm/multisig.wasm
```

5. Upgrade Multisig contract

There is no state migration needed during upgrade.

```bash
node cosmwasm/submit-proposal.js migrate \
  -c Multisig \
  -t "Migrate Multisig to v1.2.0" \
  -d "Multisig to v1.2.0" \
  --msg '{}' \
  --fetchCodeId \
  --deposit 2000000000
```

## Checklist

Verify multisig contract version

```bash
axelard query wasm contract-state raw $MULTISIG_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```
Expected output

```bash
{"contract":"multisig","version":"1.2.0"}
```


