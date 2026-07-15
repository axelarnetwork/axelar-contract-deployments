# XRPL Amplifier — XRPLMultisigProver v1.4.3 & XRPLGateway v1.3.3

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @Deiadara <nikos@commonprefix.com> |
| **Deployment Mainnet** | @themicp <themis@commonprefix.com> |
| **Deployment Others** | @dolfje <nikosver@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | Deployed              | 2026-07-13 |
| **Stagenet**         | Deployed              | 2026-07-13 |
| **Testnet**          | Deployed              | 2026-07-15 |
| **Mainnet**          | Deployed              | 2026-06-29 |

Releases:

- [XRPLMultisigProver v1.4.3](https://github.com/axelarnetwork/axelar-amplifier-xrpl/releases/tag/xrpl-multisig-prover-v1.4.3)
- [XRPLGateway v1.3.3](https://github.com/axelarnetwork/axelar-amplifier-xrpl/releases/tag/xrpl-gateway-v1.3.3)

## Background

Changes in this release:

1. XRPLMultisigProver: `compute_xrpl_amount` now canonicalizes the inbound amount with the **XRPL** token-instance decimals
1. XRPLGateway: `translate_to_interchain_transfer` now scales the outbound amount with the **XRPL** token-instance decimals

## Deployment

- This rollout upgrades XRPLMultisigProver from `v1.4.2` to `v1.4.3` and XRPLGateway from `v1.3.2` to `v1.3.3`
- There is no state migration involved, i.e., each migrate step just updates the code

| Network              | XRPLMultisigProver                                                  | XRPLGateway                                                         |
| -------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| **Devnet Amplifier** | `axelar1ys83sedjffmqh70aksejmx3fy3q2d7twm3msurk7wn3l6nkwxp0sfelzhl`                                                            | ` axelar1fqy77ptzsspmewy547dappss2j7scy9asju8qqjts67r2tl4k5cqg6zfxa`                                                            |
| **Stagenet**         | `axelar17mnfwue8mw5t2q9gmndz3m50070fmm0ffa0tunh2dujmuy398rtsrjyw2k`                                                            | `axelar1vr89h7je4zw4hxhdt7aycqq6end3kf9wvj4unv63s8pcenrrutrs88kn57`                                                            |
| **Testnet**          | `axelar1k82qfzu3l6rvc7twlp9lpwsnav507czl6xyrk0xv287t4439ymvsl6n470`                                                            | `axelar18qltw4382s5qz0rgzfxz67mr83smk580hewlkfd33l5tmcdp8unqw35glh`                                                            |
| **Mainnet**          | `axelar15mhhuf887t6nfx2t0vuc6kx9w2uk65h939awmz6n7r6ggzyf659st25hff` | `axelar15dsw0qqnvumnsukjtwt040wnelwrglgslqcqsa7d62f2neuv7slq7rq7zd` |

All four transactions are issued by the Emergency Committee account (`axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m`, 3/6 threshold), which is the admin of the XRPL contracts. Each tx is generated with `--generate-only`, signed offline by the CPI signers (`--sign-mode amino-json`), combined (≥3/6), and broadcast in the sequence order below. The coordinator generates the four unsigned txs and shares them with the signers out-of-band; each signer verifies, signs offline, and returns their signature file; once at least three signatures are collected, the coordinator combines and broadcasts them.

### 1. Environment and sequence numbers

The four transactions consume four consecutive sequence numbers. Fetch the on-chain sequence and derive the rest from it — the account's `sequence` field is the **next** sequence to use, so the first tx uses `$SEQ` as-is. This assumes the four txs are generated, signed, and broadcast back-to-back right before deployment, with no other `cpi-multisig` transaction landing in between.

```bash
MULTISIG=axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m
NODE=https://axelar-rpc.qubelabs.io:443

# your signer + the contracts/code ids the steps below reference (mainnet shown)
SIGNER_KEY=axelar_key
SIGNER_NAME=nikos
PROVER_ADDR=axelar15mhhuf887t6nfx2t0vuc6kx9w2uk65h939awmz6n7r6ggzyf659st25hff
GATEWAY_ADDR=axelar15dsw0qqnvumnsukjtwt040wnelwrglgslqcqsa7d62f2neuv7slq7rq7zd
PROVER_CODE=67
GATEWAY_CODE=68

ACCOUNT=$(axelard query auth account "$MULTISIG" --node "$NODE" --output json)
ACCOUNT_NUMBER=$(echo "$ACCOUNT" | jq -r '.account.value.account_number // .account.account_number')
SEQ=$(echo "$ACCOUNT" | jq -r '.account.value.sequence // .account.sequence')
```

Each release consumes two sequences (store + migrate), so the second release's store starts two above the first. Bump `$SEQ` by one per transaction, in this order:

| Tx                         | Sequence          |
| -------------------------- | ----------------- |
| Store XRPLMultisigProver   | `$SEQ`            |
| Migrate XRPLMultisigProver | `$((SEQ + 1))`    |
| Store XRPLGateway          | `$((SEQ + 2))`    |
| Migrate XRPLGateway        | `$((SEQ + 3))`    |

```bash
SEQ_PROVER_STORE=$SEQ
SEQ_PROVER_MIGRATE=$((SEQ + 1))
SEQ_GATEWAY_STORE=$((SEQ + 2))
SEQ_GATEWAY_MIGRATE=$((SEQ + 3))
```

### 2. Store contracts (generate the unsigned txs)

`xrpl_multisig_prover.wasm` and `xrpl_gateway.wasm` are the released bytecode from this version's release artifacts of the github action (verified against the published `checksums.txt`). Before signing, every signer should verify that the bytecode embedded in each `unsigned.json` is exactly the contract they expect. `body.messages[0].wasm_byte_code` is a gzip-compressed base64 string, so base64-decode and gunzip it before computing sha256:

```bash
jq -r '.body.messages[0].wasm_byte_code' unsigned.json | base64 -d | gunzip | shasum -a 256
```

The released bytecode is published to the public R2 bucket, alongside a `checksums.txt` in each directory:
- XRPLGateway v1.3.3: https://pub-7233af746dc8432f8d9547af0133309d.r2.dev/releases/cosmwasm/xrpl-gateway/1.3.3/xrpl_gateway.wasm
- XRPLMultisigProver v1.4.3: https://pub-7233af746dc8432f8d9547af0133309d.r2.dev/releases/cosmwasm/xrpl-multisig-prover/1.4.3/xrpl_multisig_prover.wasm

Confirm the computed hash matches the contract's line in the artifact `checksums.txt` (and, optionally, the hash recomputed from the downloaded `.wasm`):

```
5e70d6bcbe114710dc0735e1366374eb7c00004b9a23715191834f1f3a8a0d81  xrpl_multisig_prover.wasm
2f9dceca04c987b8fe4cb78f1f776d6f7d025cb541f4178a0ffc4054a6e79c06  xrpl_gateway.wasm
```

```bash
axelard tx wasm store xrpl_multisig_prover.wasm \
  --from cpi-multisig --chain-id axelar-dojo-1 \
  --gas 25000000 --gas-prices 0.007uaxl \
  --generate-only

axelard tx wasm store xrpl_gateway.wasm \
  --from cpi-multisig --chain-id axelar-dojo-1 \
  --gas 25000000 --gas-prices 0.007uaxl \
  --generate-only
```

### 3. Migrate contracts (generate the unsigned txs)

`$PROVER_ADDR` / `$GATEWAY_ADDR` are the deployed contracts for the target network (see the address table above); `$PROVER_CODE` / `$GATEWAY_CODE` are the code ids created by their stores in step 2.

```bash
axelard tx wasm migrate "$PROVER_ADDR" "$PROVER_CODE" '{}' \
  --from cpi-multisig --chain-id axelar-dojo-1 \
  --gas 5000000 --gas-prices 0.007uaxl \
  --generate-only

axelard tx wasm migrate "$GATEWAY_ADDR" "$GATEWAY_CODE" '{}' \
  --from cpi-multisig --chain-id axelar-dojo-1 \
  --gas 5000000 --gas-prices 0.007uaxl \
  --generate-only
```

### 4. Sign

Each signer signs all four unsigned txs offline and returns their signature file. Use `--sign-mode amino-json`, `--account-number "$ACCOUNT_NUMBER"`, and the tx's sequence (`$SEQ_TX`) from the table above:

```bash
axelard tx sign "signatures/$SESSION/unsigned.json" \
  --from "$SIGNER_KEY" \
  --multisig cpi-multisig \
  --chain-id axelar-dojo-1 \
  --account-number "$ACCOUNT_NUMBER" \
  --sequence "$SEQ_TX" \
  --sign-mode amino-json \
  --offline \
  --output-document "signatures/$SESSION/$SIGNER_NAME.signature.json"
```

### 5. Combine and broadcast

Combine ≥3 signatures and broadcast — once per tx, **in sequence order** (`$SESSION` is the signing-session dir, `$SEQ_TX` the tx's sequence from step 1):

```bash
axelard tx multisign "signatures/$SESSION/unsigned.json" cpi-multisig \
  "signatures/$SESSION/signer-a.signature.json" \
  "signatures/$SESSION/signer-b.signature.json" \
  "signatures/$SESSION/signer-c.signature.json" \
  --chain-id axelar-dojo-1 --account-number "$ACCOUNT_NUMBER" --sequence "$SEQ_TX" --offline \
  --keyring-backend "$KEYRING_BACKEND" \
  --output-document "signatures/$SESSION/signed.json"

axelard tx broadcast "signatures/$SESSION/signed.json" \
  --node "$NODE" --chain-id axelar-dojo-1 --output json
```

**Broadcast strictly in sequence order — store prover (`$SEQ`) → migrate prover (`$SEQ+1`) → store gateway (`$SEQ+2`) → migrate gateway (`$SEQ+3`)** — each landing in a block before the next is broadcast (every migrate needs the code id created by its store, and each broadcast consumes the next account sequence).

## Checklist

Verify contract versions (use the target network's addresses from the table above):

```bash
axelard query wasm contract-state raw "$PROVER_ADDR" 636F6E74726163745F696E666F --node "$NODE" -o json | jq -r '.data' | base64 -d
axelard query wasm contract-state raw "$GATEWAY_ADDR" 636F6E74726163745F696E666F --node "$NODE" -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"xrpl-multisig-prover","version":"1.4.3"}
{"contract":"xrpl-gateway","version":"1.3.3"}
```

Follow the [XRPL checklist](../xrpl/2025-02-v1.0.0.md) to ensure that all flows are still functioning as expected.

## Testing

Beyond the version check, run end-to-end transfers on the target network. These exercise exactly the inbound (prover) and outbound (gateway) decimal scaling this release changes, so check the **delivered amounts**, not just that the transfers succeed.

- **XRPL ↔ XRPL-EVM ITS — via the `axe` repo.** Send an ITS transfer of XRP from XRPL → XRPL-EVM, and another from XRPL-EVM → XRPL. This directly covers incoming and outgoing XRPL traffic.
- **Full cross-chain route — via `axelar-contract-deployments`, `axe` and the Squid frontend.** Verify incoming and outgoing XRP and IOU transfers by running tests through [AXE](https://github.com/axelarnetwork/axe), then run complete routes like **XRPL → ETH → Solana → XRPL** and **XRPL → SOLANA → ETH → XRPL** and confirm every hop settles and the balances reconcile. Make sure to test XRPL between every Amplifier chain type (Solana, Stellar, Hedera, XRPL-EVM, Sui).

### Script to send from Solana to XRPL
This is how to send token transfers from Solana to XRPL:
```
TOKEN_ID=...
DESTINATION_ADDRESS=...
AMOUNT=0.01
GAS_VALUE=3800000

DESTINATION_ADDRESS_BYTES=$(printf '%s' "$DESTINATION_ADDRESS" | xxd -p | tr -d '\n')
./solana/cli send its interchain-transfer \
    --token-id $TOKEN_ID \
    --destination-chain xrpl \
    --destination-address $DESTINATION_ADDRESS_BYTES \
    --amount $AMOUNT \
    --gas-value $GAS_VALUE
```
_Note: You will need to build the rust binary first_

### Script to send from XRPL to other chains
```
TOKEN_ADDRESS=xyz.rfmS3zqrQrka8wVyhXifEeyTwe8AMz2Yhw
DESTINATION_ADDRESS=...
DESTINATION_CHAIN=...
AMOUNT=50
GAS_FEE_AMOUNT=40 # $GAS_FEE_AMOUNT must be smaller than $AMOUNT
PRIVATE_KEY=s...

ts-node xrpl/interchain-transfer.js -n xrpl $TOKEN_ADDRESS $AMOUNT $DESTINATION_CHAIN $DESTINATION_ADDRESS -p $PRIVATE_KEY --gasFeeAmount $GAS_FEE_AMOUNT
```


## Emergency Execution Disabling

If need be, freeze the XRP Ledger integration by disabling execution on the XRPLGateway and XRPLMultisigProver contracts.

```sh
XRPL_GATEWAY="axelar15dsw0qqnvumnsukjtwt040wnelwrglgslqcqsa7d62f2neuv7slq7rq7zd"
XRPL_MULTISIG_PROVER="axelar15mhhuf887t6nfx2t0vuc6kx9w2uk65h939awmz6n7r6ggzyf659st25hff"
AXELAR_RPC_URL="https://axelar-rpc.qubelabs.io:443"
AXELAR_CHAIN_ID="axelar-dojo-1"
ADMIN_KEY_NAME="xrpl-admin"
axelard keys add $ADMIN_KEY_NAME --recover --keyring-backend os
axelard tx wasm execute $XRPL_GATEWAY '"disable_execution"' \
  --from $ADMIN_KEY_NAME --keyring-backend os \
  --chain-id $AXELAR_CHAIN_ID --node $AXELAR_RPC_URL \
  --gas 500000 --fees 3500uaxl \
  --note "xrpl killswitch: disable gateway"
axelard tx wasm execute $XRPL_MULTISIG_PROVER '"disable_execution"' \
  --from $ADMIN_KEY_NAME --keyring-backend os \
  --chain-id axelar-dojo-1 --node $AXELAR_RPC_URL \
  --gas 500000 --fees 3500uaxl \
  --note "xrpl killswitch: disable multisig-prover"
axelard q wasm contract-state smart $XRPL_GATEWAY '"is_enabled"' --node $AXELAR_RPC_URL --output json
axelard q wasm contract-state smart $XRPL_MULTISIG_PROVER '"is_enabled"' --node $AXELAR_RPC_URL --output json
axelard keys delete $ADMIN_KEY_NAME --keyring-backend os
```

# devnet-amplifier deployment details

For devnet we will use to upgrade with the admin key instead of governance. As that is the default for previous upgrades.

Create .env file

```
MNEMONIC="..."
ENV=devnet-amplifier
CHAIN=xrpl-dev
RELEASES_BASE_URL=https://pub-7233af746dc8432f8d9547af0133309d.r2.dev
ARTIFACT_PATH=wasm

_ADDRESSES=axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9
PROVER_ADMIN=axelar1lsasewgqj7698e9a25v3c9kkzweee9cvejq5cs
```

## XrplMultisigProver 1.4.2

devnet-amplifier was still on 1.4.1, so first the 1.4.2 upgrade was done.

1. Download and check release

```
source .env

mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.2/xrpl_multisig_prover.wasm --directory-prefix=$ARTIFACT_PATH

wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.2/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_multisig_prover.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_multisig_prover.wasm | grep $CHECKSUM
```

2. Upload new contract

```
npx ts-node cosmwasm/contract.ts store-code   -c XrplMultisigProver   -e $ENV -n $CHAIN   -a $ARTIFACT_PATH -t "Upload XRPLMultisigProver contract v1.4.2"   -d "Upload XRPLMultisigProver contract v1.4.2" --instantiateAddresses $INIT_ADDRESSES

# Uploading contract binary
# 
# Uploaded contract binary with codeId: 1652
```

3. Upgrade contract

```
npx ts-node cosmwasm/contract.ts migrate   -c XrplMultisigProver   --msg '{}'   --codeId 1652 -e $ENV -n $CHAIN

# Using code id: 1652
# 
# Proceed with contract migration on axelar? (y/n) y
# 
# Migration completed. Transaction hash: CB4DBE7B23718876FD9D79665342EB2CFE3FE7F853FFF7467514A1B345CEF189
```

4. Check upgrade

```

# {"contract":"xrpl-multisig-prover","version":"1.4.2"}
```

## XrplMultisigProver 1.4.3

1. Download and check release

```
mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.3/xrpl_multisig_prover.wasm --directory-prefix=$ARTIFACT_PATH

wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-multisig-prover/1.4.3/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_multisig_prover.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_multisig_prover.wasm | grep $CHECKSUM

# 5e70d6bcbe114710dc0735e1366374eb7c00004b9a23715191834f1f3a8a0d81  wasm/xrpl_multisig_prover.wasm
```

2. Upload new contract

```
npx ts-node cosmwasm/contract.ts store-code   -c XrplMultisigProver   -e $ENV -n $CHAIN   -a $ARTIFACT_PATH -t "Upload XRPLMultisigProver contract v1.4.3"   -d "Upload XRPLMultisigProver contract v1.4.3" --instantiateAddresses $INIT_ADDRESSES

# Uploading contract binary
# 
# Uploaded contract binary with codeId: 1653
```

3. Upgrade contract

```
npx ts-node cosmwasm/contract.ts migrate   -c XrplMultisigProver   --msg '{}'   --codeId 1653 -e $ENV -n $CHAIN

# Using code id: 1653
# 
# Proceed with contract migration on axelar? (y/n) y
# 
# Migration completed. Transaction hash: DADE20CB72D4082BB65CDE5A936F290EA26C7306CB98C556A6557552B661EA1F
```

4. Check upgrade

```
# {"contract":"xrpl-multisig-prover","version":"1.4.3"}
```

## XrplGateway 1.3.3

1. Download and check release

```
mkdir $ARTIFACT_PATH
wget $RELEASES_BASE_URL/releases/cosmwasm/xrpl-gateway/1.3.3/xrpl_gateway.wasm --directory-prefix=$ARTIFACT_PATH

wget -O checksums.txt $RELEASES_BASE_URL/releases/cosmwasm/xrpl-gateway/1.3.3/checksums.txt
CHECKSUM=$(cat checksums.txt | grep xrpl_gateway.wasm | awk '{print $1}')
shasum -a 256 $ARTIFACT_PATH/xrpl_gateway.wasm | grep $CHECKSUM

# 2f9dceca04c987b8fe4cb78f1f776d6f7d025cb541f4178a0ffc4054a6e79c06  wasm/xrpl_gateway.wasm
```

2. Upload new contract

```
npx ts-node cosmwasm/contract.ts store-code   -c XrplGateway   -e $ENV -n $CHAIN   -a $ARTIFACT_PATH -t "Upload XRPLGateway contract v1.3.3"   -d "Upload XRPLGateway contract v1.3.3" --instantiateAddresses $INIT_ADDRESSES

# Uploading contract binary
# 
# Uploaded contract binary with codeId: 1654
```

3. Upgrade contract

```
npx ts-node cosmwasm/contract.ts migrate   -c XrplGateway   --msg '{}'   --codeId 1654 -e $ENV -n $CHAIN

# Using code id: 1654
# 
# Proceed with contract migration on axelar? (y/n) y
# 
# Migration completed. Transaction hash: 755D7CC4D74F1A849954AB229A6B177C9AE9429ABF5A7C8E4A402D172FE891F5
```

# stagenet deployment details

1. Create wallet

```
node -e 'const {DirectSecp256k1HdWallet}=require("@cosmjs/proto-signing");(async()=>{const w=await DirectSecp256k1HdWallet.generate(24,{prefix:"axelar"});console.log(w.mnemonic);console.log((await w.getAccounts())[0].address)})()'
```

2. Create .env file

```
MNEMONIC="..."
ADDR="axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c" // from step 1
ENV=stagenet
CHAIN=xrpl
RELEASES_BASE_URL=https://pub-7233af746dc8432f8d9547af0133309d.r2.dev
ARTIFACT_PATH=wasm
```

2. Fund wallet from validators

3. Upload new contracts

```
npx tsx ts-node cosmwasm/contract.ts store-code -c XrplMultisigProver -e $ENV -n $CHAIN -a "$ARTIFACT_PATH" \
  --governance -t "Store XRPLMultisigProver v1.4.3" -d "Store XRPLMultisigProver v1.4.3"
 
#Encoded /cosmwasm.wasm.v1.MsgStoreCode: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "wasmByteCode": "<783423 bytes>",
#  "instantiatePermission": {
#    "permission": 4,
#    "addresses": [
#      "axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm",
#      "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#      "axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky"
#    ]
#  }
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 505
```

```
npx ts-node cosmwasm/contract.ts store-code -c XrplGateway        -e $ENV -n $CHAIN -a "$ARTIFACT_PATH" \
  --governance -t "Store XRPLGateway v1.3.3" -d "Store XRPLGateway v1.3.3"

#Encoded /cosmwasm.wasm.v1.MsgStoreCode: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "wasmByteCode": "<610002 bytes>",
#  "instantiatePermission": {
#    "permission": 4,
#    "addresses": [
#      "axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm",
#      "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#      "axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky"
#    ]
#  }
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 506
```

4. upgrade contracts

```
npx ts-node cosmwasm/contract.ts migrate -c XrplGateway        -e $ENV -n $CHAIN --governance --fetchCodeId --msg '{}'

#Fetched code id 101 from the network
#
#Encoded /cosmwasm.wasm.v1.MsgMigrateContract: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "contract": "axelar1vr89h7je4zw4hxhdt7aycqq6end3kf9wvj4unv63s8pcenrrutrs88kn57",
#  "codeId": "101",
#  "msg": {}
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 507
```

```
npx ts-node cosmwasm/contract.ts migrate -c XrplMultisigProver -e $ENV -n $CHAIN --governance --fetchCodeId --msg '{}'

#Fetched code id 100 from the network
#
#Encoded /cosmwasm.wasm.v1.MsgMigrateContract: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "contract": "axelar17mnfwue8mw5t2q9gmndz3m50070fmm0ffa0tunh2dujmuy398rtsrjyw2k",
#  "codeId": "100",
#  "msg": {}
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 508
```

# testnet deployment details

1. Create wallet

```
node -e 'const {DirectSecp256k1HdWallet}=require("@cosmjs/proto-signing");(async()=>{const w=await DirectSecp256k1HdWallet.generate(24,{prefix:"axelar"});console.log(w.mnemonic);console.log((await w.getAccounts())[0].address)})()'
```

2. Create .env file

```
MNEMONIC="..."
ADDR="axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c" // from step 1
ENV=testnet
CHAIN=xrpl
RELEASES_BASE_URL=https://pub-7233af746dc8432f8d9547af0133309d.r2.dev
ARTIFACT_PATH=wasm
```

2. Fund wallet from faucet

3. Upload new contracts

```
npx ts-node cosmwasm/contract.ts store-code -c XrplMultisigProver -e $ENV -n $CHAIN -a "$ARTIFACT_PATH" \
  --governance -t "Store XRPLMultisigProver v1.4.3" -d "Store XRPLMultisigProver v1.4.3" -u http://localhost:26657

#Encoded /cosmwasm.wasm.v1.MsgStoreCode: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "wasmByteCode": "<785984 bytes>",
#  "instantiatePermission": {
#    "permission": 4,
#    "addresses": [
#      "axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2",
#      "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#      "axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7"
#    ]
#  }
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 625
```

```
npx ts-node cosmwasm/contract.ts store-code -c XrplGateway        -e $ENV -n $CHAIN -a "$ARTIFACT_PATH" \
  --governance -t "Store XRPLGateway v1.3.3" -d "Store XRPLGateway v1.3.3" -u http://localhost:26657
  
#Encoded /cosmwasm.wasm.v1.MsgStoreCode: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "wasmByteCode": "<612127 bytes>",
#  "instantiatePermission": {
#    "permission": 4,
#    "addresses": [
#      "axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2",
#      "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#      "axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7"
#    ]
#  }
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 626
```

4. vote the contracts in 

```
aws-vault exec commonprefix
bash vote.sh testnet 625
bash vote.sh testnet 626
```

5. upgrade contracts

```
npx ts-node cosmwasm/contract.ts migrate -c XrplGateway        -e $ENV -n $CHAIN --governance --fetchCodeId --msg '{}' -u http://localhost:26657

#Fetched code id 100 from the network
#
#Encoded /cosmwasm.wasm.v1.MsgMigrateContract: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "contract": "axelar18qltw4382s5qz0rgzfxz67mr83smk580hewlkfd33l5tmcdp8unqw35glh",
#  "codeId": "100",
#  "msg": {}
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 627
```

```
npx ts-node cosmwasm/contract.ts migrate -c XrplMultisigProver -e $ENV -n $CHAIN --governance --fetchCodeId --msg '{}' -u http://localhost:26657

#Fetched code id 99 from the network
#
#Encoded /cosmwasm.wasm.v1.MsgMigrateContract: {
#  "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
#  "contract": "axelar1k82qfzu3l6rvc7twlp9lpwsnav507czl6xyrk0xv287t4439ymvsl6n470",
#  "codeId": "99",
#  "msg": {}
#}
#
#Proceed with proposal submission? (y/n) y
#
#Proposer address: axelar1antyf0ypw656xssjtv25r95g88yq5xrx0cpq5c
#
#Proposal submitted: 628
```
6. vote the upgrade in

```
aws-vault exec commonprefix
bash vote.sh testnet 627
bash vote.sh testnet 628
```
