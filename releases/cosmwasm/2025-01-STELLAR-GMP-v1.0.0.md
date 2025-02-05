# DEPLOY COSMWASM FOR STELLAR GMP v1.0.0

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

## Background

1. This is Cosmwasm Deployment for Stellar GMP v1.0.0.

## Deployment

Create an `.env` config.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAIN=stellar
```

## Deployment

- This rollout will create a new stellar connection

```bash
GATEWAY_ADDRESS=gateway
PROVER_ADDRESS=prover
RUN_AS_ACCOUNT=run_as_account
REWARD_POOL=reward_pool
VERIFY_REWARD_POOL=verify_reward_pool
MULTISIG_REWARD_POOL=multisig_reward_pool
DEPOSIT_VALUE=100000000
```

```bash
# Instantiate stellar VotingVerifier
node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier -n $CHAIN --fetchCodeId --instantiate2

# Instantiate stellar gateway
node ./cosmwasm/deploy-contract.js instantiate -c Gateway -n $CHAIN --fetchCodeId --instantiate2

# Instantiate stellar MultisigProver
node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver -n $CHAIN --fetchCodeId --instantiate2

# Register stellar gateway at the Router
node cosmwasm/submit-proposal.js execute \
  -c Router \
  -t "Register Gateway for stellar" \
  -d "Register Gateway address for stellar at Router contract" \
  --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg '{"register_chain":{"chain":"'"$CHAIN"'","gateway_address":"'"$GATEWAY_ADDRESS"'","msg_id_format":"hex_tx_hash_and_event_index"}}'

# Register prover contract on coordinator
node cosmwasm/submit-proposal.js execute \
  -c Coordinator \
  -t "Register Multisig Prover for stellar" \
  -d "Register Multisig Prover address for stellar at Coordinator contract" \
  --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg '{"register_prover_contract":{"chain_name":"'"$CHAIN"'","new_prover_addr":""'"$PROVER_ADDRESS"'""}}'

# Authorize callers on multisig prover
node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Authorize Multisig Prover for stellar" \
  -d "Authorize Multisig Prover address for stellar at Multisig contract" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg '{"authorize_callers":{"contracts":{"'"$PROVER_ADDRESS"'":"'"$CHAIN"'"}}}'

# Update verifier set
axelard tx wasm execute "'"$PROVER_ADDRESS"'" '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 1.2
```

Create reward Pools and add funds to reward pools

```bash
# Create verify reward pool
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
      "contract": "'"$VERIFY_REWARD_POOL"'"
    }
  }
}'

# Create multisig reward pool
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
      "contract": "'"$MULTISIG_REWARD_POOL"'"
    }
  }
}'

# Add funds to reward pools
echo $KEYRING_PASSWORD | axelard tx wasm execute $REWARD_POOL '{"add_rewards":{"pool_id":{"chain_name":"'"$CHAIN"'","contract":"'"$MULTISIG_REWARD_POOL"'"}}}' --amount 1000000uamplifier --from validator

echo $KEYRING_PASSWORD | axelard tx wasm execute $REWARD_POOL '{"add_rewards":{"pool_id":{"chain_name":"'"$CHAIN"'","contract":"'"$VERIFY_REWARD_POOL"'"}}}' --amount 1000000uamplifier --from validator
```

## Checklist

The [Stellar GMP checklist](../stellar/2025-01-GMP-v1.0.0.md) will test GMP call.
