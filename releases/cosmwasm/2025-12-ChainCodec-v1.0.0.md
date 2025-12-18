# Cosmwasm ChainCodec v1.0.0

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @chipshort <christoph@interoplabs.io> |
| **Deployment** |                                       |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

[Release (TBD)](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/chain-codec-v1.0.0)

## Background

These are the instructions for deploying the new ChainCodec Amplifier contract and
migrating the MultisigProver, VotingVerifier and Coordinator to latest version.

## Deployment

### Preparation

Setup your `.env` config.

```yaml
MNEMONIC=xyz
ENV=xyz
ARTIFACT_PATH=wasm
INIT_ADDRESSES=xyz
```

The [ChainCodec deployment](#chaincodec-deployment) and [MultisigProver and VotingVerifier migration](#multisigprover-and-votingverifier-migration) steps have to be done sequentially, but the [Coordinator migration](#coordinator-migration) can be done independently.

### ChainCodec deployment

1. Upload and instantiate new ChainCodec* contracts. Depending on the network, you can either upload and instantiate directly using the usual scripts (if you have the governance key) or submit a proposal to the network like this:

    ```bash
    ts-node cosmwasm/migrate/chain-codec.ts store-instantiate-chain-codecs \
        -t "Store and instantiate chain-codec contracts" \
        -d "stores and instantiates chain-codec v1.0.0 for sui, stellar and evm" \
        -a "$ARTIFACT_PATH" \
        --instantiateAddresses $INIT_ADDRESSES \
        -c ChainCodecSui ChainCodecStellar ChainCodecEvm
    ```
    
    When the proposal passed, get the `codeId`s and `address`es for the ChainCodec contracts from the network and add them to the config:

    ```bash
    RPC=$(cat ./axelar-chains-config/info/$ENV.json | jq -r '.axelar.rpc')
    HASHES=($(cat ./axelar-chains-config/info/$ENV.json | jq -r '.axelar.contracts.ChainCodecSui.storeCodeProposalCodeHash + " " + .axelar.contracts.ChainCodecStellar.storeCodeProposalCodeHash + " " + .axelar.contracts.ChainCodecEvm.storeCodeProposalCodeHash'))

    CODE_JSON=$(axelard q --node "$RPC" wasm list-code --reverse -o json)
    for HASH in $HASHES; do
        echo "Hash: $HASH"
        CODE_ID=$(echo "$CODE_JSON" | jq -r '.code_infos[] | select(.data_hash | ascii_downcase == "'$HASH'").code_id')
        echo "Code ID: $CODE_ID"
        ADDRESS=$(axelard q --node "$RPC" wasm list-contract-by-code $CODE_ID -o json | jq -r '.contracts[0]')
        echo "Address: $ADDRESS"
        echo "---"
    done
    ```

    Add that to the config in the `codeId` and `address` fields of `ChainCodecSui`, `ChainCodecStellar` and `ChainCodecEvm`
    (choose by comparing the hashes). Example:

    ```json
    "ChainCodecSui": {
        "storeInstantiateProposalId": "4",
        "storeCodeProposalCodeHash": "633cefd1924e67d0d3124f9fa08a3f997650355aa62f4ab619449a7122f77350",
        "codeId": 19,
        "address": "axelar1vu8hcsjpacnngsqx2x4w9wjh2zl55u68nm3cv5atl4ut4dkaus4skfuy34"
    },
    "ChainCodecStellar": {
        "storeInstantiateProposalId": "4",
        "storeCodeProposalCodeHash": "0bbdcbc5b54c683e6a91f3194fad7ec9f7966d16d0d6dbf11b6efbe953f5226a",
        "codeId": 20,
        "address": "axelar14qzan3htphfmuvzugck5n8wguxtdp0z204pldwyv6rv4mnec087qk9j0z7"
    },
    "ChainCodecEvm": {
        "storeInstantiateProposalId": "4",
        "storeCodeProposalCodeHash": "5942077753689f968f1f708b406266cc099db1b4019381ad54b8b675c4afff04",
        "codeId": 21,
        "address": "axelar1800drchmd7pq8l3jdc0hpr8ngk8d9vpqqay9r07ms5kjyx34838sdeh4z9"
    }
    ```

### MultisigProver and VotingVerifier migration

1. Prepare the config for chain-codec instantiation and migration of MultisigProver and VotingVerifier

    ```bash
    ts-node cosmwasm/migrate/chain-codec.ts prepare
    ```

2. Upload new MultisigProver and VotingVerifier contracts. You need to provide a chain name for some reason,
    so just provide whatever amplifier chain name you want.
    ```bash
    ts-node cosmwasm/submit-proposal.js store \
        -t "Upload MultisigProver v1.2.0 and VotingVerifier v2.1.0 contracts" \
        -d "Upload MultisigProver v1.2.0 and VotingVerifier v2.1.0 contracts" \
        -a "$ARTIFACT_PATH" \
        -i $INIT_ADDRESSES \
        -n stellar \
        -c MultisigProver VotingVerifier
    ```
    Make sure to update the `lastUploadedCodeId` fields for MultisigProver and VotingVerifier in the config.

3. Migrate MultisigProver and VotingVerifier to latest version. You can migrate directly by passing the `--direct` flag.
    ```bash
    ts-node cosmwasm/migrate/chain-codec.ts migrate-mp-vv \
        -t "Migrate MultisigProver to v1.2.0 and VotingVerifier to v2.1.0" \
        -d "Migrate MultisigProver to v1.2.0 and VotingVerifier to v2.1.0"
    ```
    Make sure to update the `codeId` fields for MultisigProver and VotingVerifier in the config when the proposal passed.

### Coordinator migration

1. Store the new Coordinator contract.
    ```bash
    ts-node cosmwasm/submit-proposal.js store \
        -c Coordinator \
        -t "Upload Coordinator contract v3.0.0" \
        -d "Upload Coordinator contract v3.0.0" \
        -a "$ARTIFACT_PATH" \
        -i $INIT_ADDRESSES \
    ```
2. Migrate the Coordinator to the stored contract.
    ```bash
    ts-node cosmwasm/submit-proposal.js migrate \
        -c Coordinator \
        -t "Migrate Coordinator to v3.0.0" \
        -d "Migrate Coordinator to v3.0.0" \
        --msg '{}' \
        --fetchCodeId
    ```


## Checklist

Verify multisig and voting verifier contract version

```bash
axelard query wasm contract-state raw $ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected outputs

```bash
{"contract":"multisig","version":"1.2.0"}
{"contract":"voting-verifier","version":"2.1.0"}
```
