# XRPLGateway v1.3.0

|                | **Owner**                                                                                                 |
| -------------- | --------------------------------------------------------------------------------------------------------- |
| **Created By** | @k4m4 <nikolas@commonprefix.com>                                                                          |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>, @isi8787 <isaac@interoplabs.io>, @k4m4 <nikolas@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-05-19 |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/commonprefix/axelar-amplifier/releases/tag/xrpl-gateway-v1.3.0)

## Background

Changes in this release:

1. Emit ITS events

## Deployment

- This rollout upgrades XRPLGateway from `v1.2.0` to `v1.3.0`
- There is no migration involved, i.e., the migrate step will just update the code

1. Create `.env`.

| Network              | `INIT_ADDRESSES`                                                                                                                            |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| **Devnet-amplifier** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               |
| **Stagenet**         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
| **Testnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| **Mainnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |

```bash
MNEMONIC=xyz
ENV=abc
CHAIN=xrpl
RELEASES_BASE_URL=https://pub-7233af746dc8432f8d9547af0133309d.r2.dev
ARTIFACT_PATH=wasm
XRPL_GATEWAY=
INIT_ADDRESSES=
```

```bash
source .env
```

2. Download `XRPLGateway` wasm bytecode.

```bash
mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-gateway/1.3.0/xrpl_gateway.wasm --directory-prefix=$ARTIFACT_PATH
```

3. Download and verify checksum.

```bash
wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-gateway/1.3.0/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_gateway.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_gateway.wasm | grep $CHECKSUM
```

3. Make sure your output matches with the following expected output before proceeding.

```
9c626d4ab34d3e8cd7426b72ad476b8adce05bed3274ca1b35523e66bbcf7688  wasm/xrpl_gateway.wasm
```

4. Store `XRPLGateway` contract.

```bash
ts-node cosmwasm/contract.ts store-code \
  -c XrplGateway \
  -t "Upload XRPLGateway contract v1.3.0" \
  -d "Upload XRPLGatway contract v1.3.0" \
  -a "$ARTIFACT_PATH/xrpl_gateway.wasm" \
  --instantiateAddresses $INIT_ADDRESSES \
  --governance
```

6. Migrate `XRPLGateway` contract.

```bash
ts-node cosmwasm/contract.ts migrate \
  -c XrplGateway \
  -t "Migrate XRPLGateway to v1.3.0" \
  -d "Migrate XRPLGateway to v1.3.0" \
  --msg '{}' \
  --fetchCodeId \
  --governance
```

## Checklist

Verify `XRPLGateway` contract version:

```bash
axelard query wasm contract-state raw $XRPL_GATEWAY 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"xrpl-gateway","version":"1.3.0"}
```

Follow the [XRPL checklist](../xrpl/2025-02-v1.0.0.md) to ensure that all flows are still functioning as expected.
