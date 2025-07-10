# XRPLGateway v1.0.1

|                | **Owner**                                                                |
|----------------|--------------------------------------------------------------------------|
| **Created By** | @k4m4 <nikolas@commonprefix.com>                                         |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>, @k4m4 <nikolas@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**   |
|----------------------|-----------------------|------------|
| **Devnet Amplifier** | Deployed              | 2025-03-26 |
| **Stagenet**         | Deployed              | 2025-03-28 |
| **Testnet**          | Deployed              | 2025-03-28 |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/commonprefix/axelar-amplifier/releases/tag/xrpl-gateway-v1.0.1)

## Background

Changes in this release:

1. Fix bug in XRPLTokenAmount arithmetic operations

## Deployment

- This rollout upgrades XRPLGateway from `v1.0.0` to `v1.1.0`
- There is no migration involved, i.e., the migrate step will just update the code

1. Create `.env`.

| Network              | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Devnet-amplifier** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Stagenet**         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Testnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Mainnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |

| Network              | `DEPOSIT_VALUE` |
| -------------------- | --------------- |
| **Devnet-amplifier** | `100000000`     |
| **Stagenet**         | `100000000`     |
| **Testnet**          | `2000000000`    |
| **Mainnet**          | `2000000000`    |

```bash
MNEMONIC=xyz
ENV=abc
CHAIN=xrpl
RELEASES_BASE_URL=https://pub-7233af746dc8432f8d9547af0133309d.r2.dev
ARTIFACT_PATH=wasm
XRPL_GATEWAY=
INIT_ADDRESSES=
RUN_AS_ACCOUNT=
DEPOSIT_VALUE=
```

```bash
source .env
```

2. Download `XRPLGateway` wasm bytecode.

```bash
mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-gateway/1.0.1/xrpl_gateway.wasm --directory-prefix=$ARTIFACT_PATH
```

3. Download and verify checksum.

```bash
wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-gateway/1.0.1/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_gateway.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_gateway.wasm | grep $CHECKSUM
```

3. Make sure your output matches with the following expected output before proceeding.

```
695eac10df3b5e5c571047ddab1bc64b92e80938d424907d7e2039c86883fd51  wasm/xrpl_gateway.wasm
```

4. Store `XRPLGateway` contract.

```bash
ts-node cosmwasm/submit-proposal.js store \
  -c XrplGateway \
  -t "Upload XRPLGateway contract v1.0.1" \
  -d "Upload XRPLGateway contract v1.0.1" \
  -a "$ARTIFACT_PATH/xrpl_gateway.wasm" \
  --deposit $DEPOSIT_VALUE \
  --instantiateAddresses $INIT_ADDRESSES
```

6. Migrate `XRPLGateway` contract.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c XrplGateway \
  -t "Migrate XrplGateway to v1.0.1" \
  -d "Migrate XrplGateway to v1.0.1" \
  --msg '{}' \
  --fetchCodeId \
  --deposit $DEPOSIT_VALUE
```

## Checklist

Verify XRPLGateway contract version:

```bash
axelard query wasm contract-state raw $XRPL_GATEWAY 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"xrpl-gateway","version":"1.0.1"}
```

Follow the [XRPL checklist](../xrpl/2025-02-v1.0.0.md) to ensure that all flows are still functioning as expected.
