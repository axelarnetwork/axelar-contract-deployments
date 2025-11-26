# XRPLVotingVerifier v2.0.0

|                | **Owner**                                                                                                 |
|----------------|-----------------------------------------------------------------------------------------------------------|
| **Created By** | @k4m4 <nikolas@commonprefix.com>                                                                          |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>, @isi8787 <isaac@interoplabs.io>, @k4m4 <nikolas@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**   |
|----------------------|-----------------------|------------|
| **Devnet Amplifier** | Deployed              | 2025-11-25 |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/commonprefix/axelar-amplifier/releases/tag/xrpl-voting-verifier-v2.0.0)

## Background

Changes in this release:

1. Unify parameter updates with `UpdateVotingParameters`

## Deployment

- This rollout upgrades XRPLVotingVerifier from `v1.3.0` to `v2.0.0`
- There is no migration involved, i.e., the migrate step will just update the code

1. Create `.env`.

| Network              | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
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
XRPL_VOTING_VERIFIER=
INIT_ADDRESSES=
RUN_AS_ACCOUNT=
DEPOSIT_VALUE=
```

```bash
source .env
```

2. Download `XRPLVotingVerifier` wasm bytecode.

```bash
mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-voting-verifier/2.0.0/xrpl_voting_verifier.wasm --directory-prefix=$ARTIFACT_PATH
```

3. Download and verify checksum.

```bash
wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-voting-verifier/2.0.0/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_voting_verifier.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_voting_verifier.wasm | grep $CHECKSUM
```

3. Make sure your output matches with the following expected output before proceeding.

```
2abe2cbdcf0b3f983ea9121521aede7269b0b2e08787e104a7f249c217eac083  wasm/xrpl_voting_verifier.wasm
```

4. Store `XRPLVotingVerifier` contract.

```bash
ts-node cosmwasm/submit-proposal.js store \
  -c XrplVotingVerifier \
  -t "Upload XRPLVotingVerifier contract v2.0.0" \
  -d "Upload XRPLVotingVerifier contract v2.0.0" \
  -a "$ARTIFACT_PATH/xrpl_voting_verifier.wasm" \
  --deposit $DEPOSIT_VALUE \
  --instantiateAddresses $INIT_ADDRESSES
```

6. Migrate `XRPLVotingVerifier` contract.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c XrplVotingVerifier \
  -t "Migrate XRPLVotingVerifier to v2.0.0" \
  -d "Migrate XRPLVotingVerifier to v2.0.0" \
  --msg '{}' \
  --fetchCodeId \
  --deposit $DEPOSIT_VALUE
```

## Checklist

Verify `XRPLVotingVerifier` contract version:

```bash
axelard query wasm contract-state raw $XRPL_VOTING_VERIFIER 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output:

```bash
{"contract":"xrpl-voting-verifier","version":"2.0.0"}
```

Follow the [XRPL checklist](../xrpl/2025-02-v1.0.0.md) to ensure that all flows are still functioning as expected.
