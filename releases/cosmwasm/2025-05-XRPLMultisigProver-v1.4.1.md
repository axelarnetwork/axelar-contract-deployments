# XRPLMultisigProver v1.4.1

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

[Release](https://github.com/commonprefix/axelar-amplifier/releases/tag/xrpl-multisig-prover-v1.4.1)

## Background

Changes in this release:

1. Emit events upon prover message confirmation
1. Revert if interchain transfer data is not `None`

## Deployment

- This rollout upgrades XRPLMultisigProver from `v1.3.1` to `v1.4.1`
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
XRPL_MULTISIG_PROVER=
INIT_ADDRESSES=
```

```bash
source .env
```

2. Download `XRPLMultisigProver` wasm bytecode.

```bash
mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.1/xrpl_multisig_prover.wasm --directory-prefix=$ARTIFACT_PATH
```

3. Download and verify checksum.

```bash
wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.1/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_multisig_prover.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_multisig_prover.wasm | grep $CHECKSUM
```

3. Make sure your output matches with the following expected output before proceeding.

```
bee1192a8ae1d8928127bbb23e259cfadf817b930c5176cf83f7985240a7254a  wasm/xrpl_multisig_prover.wasm
```

4. Store `XRPLMultisigProver` contract.

```bash
ts-node cosmwasm/contract.ts store-code \
  -c XrplMultisigProver \
  -t "Upload XRPLMultisigProver contract v1.4.1" \
  -d "Upload XRPLMultisigProver contract v1.4.1" \
  -a "$ARTIFACT_PATH/xrpl_multisig_prover.wasm" \
  --instantiateAddresses $INIT_ADDRESSES \
  --governance
```

6. Migrate `XRPLMultisigProver` contract.

```bash
ts-node cosmwasm/contract.ts migrate \
  -c XrplMultisigProver \
  -t "Migrate XRPLMultisigProver to v1.4.1" \
  -d "Migrate XRPLMultisigProver to v1.4.1" \
  --msg '{}' \
  --fetchCodeId \
  --governance
```

## Checklist

Verify `XRPLMultisigProver` contract version:

```bash
axelard query wasm contract-state raw $XRPL_MULTISIG_PROVER 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"xrpl-multisig-prover","version":"1.4.1"}
```

Follow the [XRPL checklist](../xrpl/2025-02-v1.0.0.md) to ensure that all flows are still functioning as expected.
