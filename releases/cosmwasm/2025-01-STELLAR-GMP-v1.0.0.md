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

- Ensure that [External Gateway](../stellar/2025-01-Stellar-GMP-v1.0.0.md) is deployed first, as `VotingVerifier` relies on `External Gateway`, due to `sourceGatewayAddress`.

## Deployment

Create an `.env` config. `CHAIN` should be set to `stellar` for mainnet, and `stellar-2024-q4` for all other networks.

```yaml
MNEMONIC=xyz
ENV=xyz
CHAIN=xyz
```

### Deploy Amplifier contracts that connect to the Stellar gateway.

```bash
GATEWAY_ADDRESS=gateway
PROVER_ADDRESS=prover
RUN_AS_ACCOUNT=run_as_account
REWARD_POOL=reward_pool
VERIFY_REWARD_POOL=verify_reward_pool
MULTISIG_REWARD_POOL=multisig_reward_pool
DEPOSIT_VALUE=100000000
REWARD_AMOUNT=1000000uamplifier
```

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

4. Register stellar gateway at the Router

```bash
node cosmwasm/submit-proposal.js execute \
 -c Router \
 -t "Register Gateway for stellar" \
 -d "Register Gateway address for stellar at Router contract" \
 --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg '{"register_chain":{"chain":"'"$CHAIN"'","gateway_address":"'"$GATEWAY_ADDRESS"'","msg_id_format":"hex_tx_hash_and_event_index"}}'
```

5. Register prover contract on coordinator

```bash
node cosmwasm/submit-proposal.js execute \
 -c Coordinator \
 -t "Register Multisig Prover for stellar" \
 -d "Register Multisig Prover address for stellar at Coordinator contract" \
 --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg '{"register_prover_contract":{"chain_name":"'"$CHAIN"'","new_prover_addr":""'"$PROVER_ADDRESS"'""}}'
```

6. Authorize callers on multisig prover

```bash
node cosmwasm/submit-proposal.js execute \
 -c Multisig \
 -t "Authorize Multisig Prover for stellar" \
 -d "Authorize Multisig Prover address for stellar at Multisig contract" \
 --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg '{"authorize_callers":{"contracts":{"'"$PROVER_ADDRESS"'":"'"$CHAIN"'"}}}'
```

7. Update verifier set

```bash
axelard tx wasm execute "'"$PROVER_ADDRESS"'" '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 1.2
```

### Create reward Pools and add funds to reward pools

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
      "contract": "'"$VERIFY_REWARD_POOL"'"
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
      "contract": "'"$MULTISIG_REWARD_POOL"'"
    }
  }
}'
```

10. Add funds to reward pools

```bash
axelard tx wasm execute $REWARD_POOL '{"add_rewards":{"pool_id":{"chain_name":"'"$CHAIN"'","contract":"'"$MULTISIG_REWARD_POOL"'"}}}' --amount $REWARD_AMOUNT --from $WALLET

axelard tx wasm execute $REWARD_POOL '{"add_rewards":{"pool_id":{"chain_name":"'"$CHAIN"'","contract":"'"$VERIFY_REWARD_POOL"'"}}}' --amount $REWARD_AMOUNT --from $WALLET
```

## Checklist

The [Stellar GMP checklist](../stellar/2025-01-GMP-v1.0.0.md) will test GMP call.
