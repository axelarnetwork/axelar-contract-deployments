# Stellar GMP Amplifier

|                | **Owner**                            |
| -------------- | ------------------------------------ |
| **Created By** | @ahramy <ahram@interoplabs.io>       |
| **Deployment** | @RiceAndMeet <steven@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases)

## Background

1. These are the instructions for deploying Cooswasm for Stellar GMP V1.0.0
2. The latest official release of axelar-amplifier should be used.

### Pre-requisite

- Ensure that [External Gateway](../stellar/2025-01-Stellar-GMP-v1.0.0.md) is deployed first, as `VotingVerifier` relies on `External Gateway` due to `sourceGatewayAddress`.

## Deployment

Create an `.env` config. `CHAIN` should be set to `stellar` for mainnet, and `stellar-2024-q4` for all other networks.

```yaml
MNEMONIC=xyz
ENV=xyz
CHAIN=xyz
```

### Deploy Amplifier contracts that connect to the Stellar gateway.

1. Instantiate stellar VotingVerifier

```bash
node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier -n $CHAIN --fetchCodeId --instantiate2
```

2. Instantiate stellar gateway

```bash
node ./cosmwasm/deploy-contract.js instantiate -c Gateway -n $CHAIN --fetchCodeId --instantiate2
```

3. Instantiate stellar MultisigProver

```bash
node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver -n $CHAIN --fetchCodeId --instantiate2

```

4. Set environment variables

```bash
VOTING_VERIFIER=$(cat ./axelar-chains-config/info/$DEVNET.json | jq ".axelar.contracts.VotingVerifier[\"$CHAIN\"].address" | tr -d '"')
GATEWAY=$(cat ./axelar-chains-config/info/$DEVNET.json | jq ".axelar.contracts.Gateway[\"$CHAIN\"].address" | tr -d '"')
MULTISIG_PROVER=$(cat ./axelar-chains-config/info/$DEVNET.json | jq ".axelar.contracts.MultisigProver[\"$CHAIN\"].address" | tr -d '"')
MULTISIG=$(cat ./axelar-chains-config/info/$DEVNET.json | jq .axelar.contracts.Multisig.address | tr -d '"')
REWARDS=$(cat ./axelar-chains-config/info/$DEVNET.json | jq .axelar.contracts.Rewards.address | tr -d '"')
RUN_AS_ACCOUNT=run_as_account
REWARD_AMOUNT=1000000uamplifier
DEPOSIT_VALUE=100000000
```

4. Register stellar gateway at the Router

```bash
node cosmwasm/submit-proposal.js execute \
 -c Router \
 -t "Register Gateway for stellar" \
 -d "Register Gateway address for stellar at Router contract" \
 --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg '{"register_chain":{"chain":"'"$CHAIN"'","gateway_address":"'"$GATEWAY"'","msg_id_format":"hex_tx_hash_and_event_index"}}'
```

5. Register prover contract on coordinator

```bash
node cosmwasm/submit-proposal.js execute \
 -c Coordinator \
 -t "Register Multisig Prover for stellar" \
 -d "Register Multisig Prover address for stellar at Coordinator contract" \
 --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg '{"register_prover_contract":{"chain_name":"'"$CHAIN"'","new_prover_addr":""'"$MULTISIG_PROVER"'""}}'
```

6. Authorize callers on multisig prover

```bash
node cosmwasm/submit-proposal.js execute \
 -c Multisig \
 -t "Authorize Multisig Prover for stellar" \
 -d "Authorize Multisig Prover address for stellar at Multisig contract" \
 --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg '{"authorize_callers":{"contracts":{"'"$MULTISIG_PROVER"'":"'"$CHAIN"'"}}}'
```

7. Update verifier set

```bash
axelard tx wasm execute "'"$MULTISIG_PROVER"'" '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 1.2
```

### Create reward pools and add funds to reward pools

| Network              | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
| -------------------- | ---------------- | ------------------------- | ------------------- |
| **Devnet-amplifier** | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Testnet**          | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Stagenet**         | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Mainnet**          | `"100"`          | `["8", "10"]`             | `"100"`             |

8. Create verify reward pool

```bash
node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in stellar voting verifier" \
  -d "Create pool for stellar in stellar voting verifier" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg '{
  "create_pool": {
    "params": {
      "epoch_duration": "100",
      "participation_threshold": [
        "8",
        "10"
      ],
      "rewards_per_epoch": "100"
    },
    "pool_id": {
      "chain_name": "'"$CHAIN"'",
      "contract": "'"$VOTING_VERIFIER"'"
    }
  }
}'
```

9. Create multisig reward pool

```bash
node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in axelar multisig" \
  -d "Create pool for stellar in axelar multisig" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg '{
  "create_pool": {
    "params": {
      "epoch_duration": "100",
      "participation_threshold": [
        "8",
        "10"
      ],
      "rewards_per_epoch": "100"
    },
    "pool_id": {
      "chain_name": "'"$CHAIN"'",
      "contract": "'"$MULTISIG"'"
    }
  }
}'
```

10. Add funds to reward pools

```bash
axelard tx wasm execute $REWARDS '{"add_rewards":{"pool_id":{"chain_name":"'"$CHAIN"'","contract":"'"$MULTISIG"'"}}}' --amount $REWARD_AMOUNT --from $WALLET

axelard tx wasm execute $REWARDS '{"add_rewards":{"pool_id":{"chain_name":"'"$CHAIN"'","contract":"'"$VOTING_VERIFIER"'"}}}' --amount $REWARD_AMOUNT --from $WALLET
```

## Checklist

The [Stellar GMP checklist](../stellar/2025-01-GMP-v1.0.0.md) will test GMP call.
