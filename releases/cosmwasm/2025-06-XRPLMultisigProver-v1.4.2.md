# XRPLMultisigProver v1.4.2

|                | **Owner**                                                                                                  |
|----------------|------------------------------------------------------------------------------------------------------------|
| **Created By** | @k4m4 <nikolas@commonprefix.com>                                                                           |
| **Deployment** | @isi8787 <isaac@interoplabs.io>, @k4m4 <nikolas@commonprefix.com>, @themicp <themis@commonprefix.com>      |

| **Network**          | **Deployment Status** | **Date**   |
|----------------------|-----------------------|------------|
| **Devnet Amplifier** | Deployed              | 2025-06-13 |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | Deployed              | 2025-06-13 |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/commonprefix/axelar-amplifier/releases/tag/xrpl-multisig-prover-v1.4.2)

## Background

Changes in this release:

1. Fix collision in `unsigned_tx_hash` preventing retries of duplicate messages, in some cases
1. Fix issue were TX fees were not being deducted from the XRP fee reserve

## Deployment

- This rollout upgrades XRPLMultisigProver from `v1.4.1` to `v1.4.2`
- There is no migration involved, i.e., the migrate step will just update the code

1. Create `.env`.

| Network              | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| **Devnet-amplifier** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Stagenet**         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Testnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Mainnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |

| Network              | `PROVER_ADMIN`                                  | `DEPOSIT_VALUE` |
| -------------------- | ----------------------------------------------- | --------------- |
| **Devnet-amplifier** | `axelar1lsasewgqj7698e9a25v3c9kkzweee9cvejq5cs` | `100000000`     |
| **Stagenet**         | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `100000000`     |
| **Testnet**          | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `2000000000`    |
| **Mainnet**          | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` | `2000000000`    |

```bash
MNEMONIC=xyz
ENV=abc
CHAIN=xrpl
RELEASES_BASE_URL=https://pub-7233af746dc8432f8d9547af0133309d.r2.dev
ARTIFACT_PATH=wasm
XRPL_MULTISIG_PROVER=
INIT_ADDRESSES=
RUN_AS_ACCOUNT=
DEPOSIT_VALUE=
PROVER_ADMIN=
```

```bash
source .env
```

2. Download `XRPLMultisigProver` wasm bytecode.

```bash
mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.2/xrpl_multisig_prover.wasm --directory-prefix=$ARTIFACT_PATH
```

3. Download and verify checksum.

```bash
wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.2/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_multisig_prover.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_multisig_prover.wasm | grep $CHECKSUM
```

3. Make sure your output matches with the following expected output before proceeding.

```
94d8bbf002b97cc586b584f7cfc12caa812bc5ee47581df209dbd8298b6b9ec5  wasm/xrpl_multisig_prover.wasm
```

4. Store `XRPLMultisigProver` contract.

```bash
ts-node cosmwasm/submit-proposal.js store \
  -c XrplMultisigProver \
  -t "Upload XRPLMultisigProver contract v1.4.2" \
  -d "Upload XRPLMultisigProver contract v1.4.2" \
  -a "$ARTIFACT_PATH/xrpl_multisig_prover.wasm" \
  --deposit $DEPOSIT_VALUE \
  --instantiateAddresses $INIT_ADDRESSES
```

6. Migrate `XRPLMultisigProver` contract.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c XrplMultisigProver \
  -t "Migrate XRPLMultisigProver to v1.4.2" \
  -d "Migrate XRPLMultisigProver to v1.4.2" \
  --msg '{}' \
  --fetchCodeId \
  --deposit $DEPOSIT_VALUE
```

7. Override XRP fee reserve on `XRPLMultisigProver`.

Once the migration has gone through, calculate the new fee reserve amount (i.e., sum of reserve top ups minus sum of proof transaction fees),
and override the `fee_reserve` on the XRPLMultisigProver with this up-to-date value.

```bash
NEW_FEE_RESERVE=[computed XRP value in drops]
axelard tx wasm execute $XRPL_MULTISIG_PROVER '{"update_fee_reserve": '$NEW_FEE_RESERVE'}' --from $PROVER_ADMIN --gas auto --gas-adjustment 1.2
```

## Checklist

Verify `XRPLMultisigProver` contract version:

```bash
axelard query wasm contract-state raw $XRPL_MULTISIG_PROVER 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"xrpl-multisig-prover","version":"1.4.2"}
```

Follow the [XRPL checklist](../xrpl/2025-02-v1.0.0.md) to ensure that all flows are still functioning as expected.
