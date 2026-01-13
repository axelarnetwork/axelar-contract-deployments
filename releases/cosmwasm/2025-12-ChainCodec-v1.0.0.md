# Cosmwasm ChainCodec v1.0.0

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @chipshort <christoph@interoplabs.io> |
| **Deployment** |                                       |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-12-23 |
| **Stagenet**         | Deployed              | 2025-12-23 |
| **Testnet**          | Deployed              | 2025-12-24 |
| **Mainnet**          | -                     | TBD        |

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
```

The [ChainCodec deployment](#chaincodec-deployment) and [MultisigProver and VotingVerifier migration](#multisigprover-and-votingverifier-migration) steps have to be done sequentially, but the [Coordinator migration](#coordinator-migration) can be done independently.

### ChainCodec deployment

1. Set the empty objects for chain codecs contracts in config

    ```json
    {
        "axelar": {
            "contracts": {
                "ChainCodecSui": {},
                "ChainCodecStellar": {},
                "ChainCodecEvm": {}
            }
        }
    }
    ```

1. Upload new chain-codec contracts.

    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -c ChainCodecSui -c ChainCodecStellar -c ChainCodecEvm \
        -t "Stores chain-codec contracts" \
        -d "Stores chain-codec v1.0.0 for Sui, Stellar and EVM" \
        --version 1.0.0 \
        --governance
    ```

1. Instantiate the chain codec contracts using instantiate2 to predict the addresses:

    ```bash
    ts-node cosmwasm/contract.ts instantiate \
        -c ChainCodecSui  \
        -t "Stores chain-codec contract for Sui" \
        -d "Stores chain-codec v1.0.0 for Sui" \
        --fetchCodeId \
        --instantiate2 \
        --governance

    ts-node cosmwasm/contract.ts instantiate \
        -c ChainCodecStellar  \
        -t "Stores chain-codec contract for Stellar" \
        -d "Stores chain-codec v1.0.0 for Stellar" \
        --fetchCodeId \
        --instantiate2 \
        --governance

    ts-node cosmwasm/contract.ts instantiate \
        -c ChainCodecEvm  \
        -t "Stores chain-codec contract EVM" \
        -d "Stores chain-codec v1.0.0 for EVM" \
        --fetchCodeId \
        --instantiate2 \
        --governance
    ```

### MultisigProver and VotingVerifier migration

1. Prepare the config for chain-codec instantiation and migration of MultisigProver and VotingVerifier

    ```bash
    ts-node cosmwasm/migrate/chain-codec.ts prepare
    ```

2. Upload new MultisigProver and VotingVerifier contracts. You need to provide a chain name for some reason,
   so just provide whatever amplifier chain name you want.

    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -t "Upload MultisigProver v1.2.0 contract" \
        -d "Upload MultisigProver v1.2.0 contract" \
        -n stellar \
        -v 1.2.0 \
        -c MultisigProver \
        --governance
    ```

    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -t "Upload VotingVerifier v2.0.1 contract" \
        -d "Upload VotingVerifier v2.0.1 contract" \
        -n stellar \
        -v 2.0.1 \
        -c VotingVerifier \
        --governance
    ```

    Wait for the proposals to pass.

3. Migrate MultisigProver and VotingVerifier to latest version. You can migrate directly by passing the `--direct` flag.
    ```bash
    ts-node cosmwasm/migrate/chain-codec.ts migrate-mp-vv \
        -t "Migrate MultisigProver to v1.2.0 and VotingVerifier to v2.0.1" \
        -d "Migrate MultisigProver to v1.2.0 and VotingVerifier to v2.0.1"
    ```
    Make sure to update the `codeId` fields for MultisigProver and VotingVerifier in the config when the proposal passed.

### Coordinator migration

1. Store the new Coordinator contract.
    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -c Coordinator \
        -t "Upload Coordinator contract v3.0.0" \
        -d "Upload Coordinator contract v3.0.0" \
        -v 3.0.0 \
        --governance
    ```
2. Migrate the Coordinator to the stored contract.
    ```bash
    ts-node cosmwasm/contract.ts migrate \
        -c Coordinator \
        -t "Migrate Coordinator to v3.0.0" \
        -d "Migrate Coordinator to v3.0.0" \
        --msg '{}' \
        --fetchCodeId \
        --governance
    ```

## Checklist

1. Verify multisig and voting verifier contract version

```bash
ts-node cosmwasm/query.ts contract-versions -c VotingVerifier MultisigProver Coordinator ChainCodecSui ChainCodecStellar ChainCodecEvm
```

Expected env config update:

```json
{
    "axelar": {
        "contracts": {
            "MultisigProver": {
                [affected-chain]: {
                    "version":"1.2.0"
                }
            },
            "VotingVerifier": {
                [affected-chain]: {
                    "version":"2.0.1"
                }
            },
            "Coordinator": {
                [affected-chain]: {
                    "version": "3.0.0"
                }
            }
        }
    }
}
```

1. Test GMP and/or ITS transfers between affected chains.
