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

- [Amplifier Releases](https://github.com/axelarnetwork/axelar-amplifier/releases)
- [VotingVerifier v1.1.0](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v1.1.0)
- [Gateway v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/gateway-v1.1.1)
- [MultisigProver v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-prover-v1.1.1)

## Background

1. These are the instructions for deploying Amplifier contracts for Stellar connection.

### Pre-requisite

- Ensure that the [External Gateway](../stellar/2025-01-Stellar-GMP-v1.0.0.md) is deployed first, as `VotingVerifier` needs the `sourceGatewayAddress` which is the External Gateway address.

## Deployment

- Create an `.env` config. `CHAIN` should be set to `stellar` for mainnet, and `stellar-2024-q4` for all other networks.

```yaml
MNEMONIC=xyz
ENV=xyz
CHAIN=xyz
```

- Confirm `VotingVerifier(v1.1.0)`, `Gateway(v1.1.1)` and `MultisigProver(v1.1.1)` contracts are already stored in `$ENV.json`

```bash
VotingVerifier(v1.1.0) -> "storeCodeProposalCodeHash": "d9412440820a51bc48bf41a77ae39cfb33101ddc6562323845627ea2042bf708"
Gateway(v1.1.1) -> "storeCodeProposalCodeHash": "2ba600ee0d162184c9387eaf6fad655f1d75db548f93e379f0565cb2042d856f"
MultisigProver(v1.1.1) -> "storeCodeProposalCodeHash": "00428ef0483f103a6e1a5853c4b29466a83e5b180cc53a00d1ff9d022bc2f03a"
```

- Add config in `$ENV.json` to deploy Amplifier contracts.

| Network              | `votingThreshold` | `signingThreshold` |
| -------------------- | ----------------- | ------------------ |
| **Devnet-amplifier** | `["6", "10"]`     | `["6", "10"]`      |
| **Stagenet**         | `["6", "10"]`     | `["6", "10"]`      |
| **Testnet**          | `["6", "10"]`     | `["6", "10"]`      |
| **Mainnet**          | `TBD`             | `TBD`              |

```bash
# Add config under "axelar": "contracts": "VotingVerifier"
$CHAIN : {
  "governanceAddress": "[governance address]",
  "serviceName": "validators",
  "sourceGatewayAddress": "[external gateway address]",
  "votingThreshold": [
    "6",
    "10"
  ],
  "blockExpiry": 10,
  "confirmationHeight": 1,
  "msgIdFormat": "hex_tx_hash_and_event_index",
  "addressFormat": "stellar"
}

# Add config under "axelar": "contracts": "MultisigProver"
$CHAIN : {
  "governanceAddress": "[governance address]]",
  "adminAddress": "[admin address]",
  "signingThreshold": [
    "6",
    "10"
  ],
  "serviceName": "validators",
  "verifierSetDiffThreshold": 0,
  "encoder": "stellar_xdr",
  "keyType": "ed25519"
}
```

### Deploy Amplifier contracts that connect to the Stellar gateway.

1. Instantiate `VotingVerifier`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2
```

2. Instantiate `Gateway`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2
```

3. Instantiate `MultisigProver`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2

```

4. Set environment variables

General environment variables

```bash
RUN_AS_ACCOUNT=[wasm deployer key address]
DEPOSIT_VALUE=100000000
REWARD_AMOUNT=1000000uamplifier
```

Network-specific environment variables: These variables need to be updated by the network.

```bash
VOTING_VERIFIER=$(cat ./axelar-chains-config/info/$DEVNET.json | jq ".axelar.contracts.VotingVerifier[\"$CHAIN\"].address" | tr -d '"')
GATEWAY=$(cat ./axelar-chains-config/info/$DEVNET.json | jq ".axelar.contracts.Gateway[\"$CHAIN\"].address" | tr -d '"')
MULTISIG_PROVER=$(cat ./axelar-chains-config/info/$DEVNET.json | jq ".axelar.contracts.MultisigProver[\"$CHAIN\"].address" | tr -d '"')
MULTISIG=$(cat ./axelar-chains-config/info/$DEVNET.json | jq .axelar.contracts.Multisig.address | tr -d '"')
REWARDS=$(cat ./axelar-chains-config/info/$DEVNET.json | jq .axelar.contracts.Rewards.address | tr -d '"')
```

5. Register stellar gateway at the Router

```bash
node cosmwasm/submit-proposal.js execute \
  -c Router \
  -t "Register Gateway for stellar" \
  -d "Register Gateway address for stellar at Router contract" \
  --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg "{
    \"register_chain\": {
      \"chain\": \"$CHAIN\",
      \"gateway_address\": \"$GATEWAY\",
      \"msg_id_format\": \"hex_tx_hash_and_event_index\"
      }
    }"
```

6. Register chain on ampd

```bash
for i in $(seq 0 4); do kubectl exec -it ampd-set-2-axelar-amplifier-worker-"$i" -n $ENV -c ampd -- ampd register-chain-support validators $CHAIN ; done

for i in $(seq 0 4); do kubectl exec -it ampd-set-2-axelar-amplifier-worker-"$i" -n $ENV -c ampd -- ampd register-public-key ed25519 ; done
```

7. Register prover contract on coordinator

```bash
node cosmwasm/submit-proposal.js execute \
  -c Coordinator \
  -t "Register Multisig Prover for stellar" \
  -d "Register Multisig Prover address for stellar at Coordinator contract" \
  --deposit $DEPOSIT_VALUE \
  --runAs $RUN_AS_ACCOUNT \
  --msg "{
    \"register_prover_contract\": {
      \"chain_name\": \"$CHAIN\",
      \"new_prover_addr\": \"$MULTISIG_PROVER\"
    }
  }"
```

8. Authorize Stellar Multisig prover on Multisig

```bash
node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Authorize Multisig Prover for stellar" \
  -d "Authorize Multisig Prover address for stellar at Multisig contract" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"authorize_callers\": {
      \"contracts\": {
        \"$MULTISIG_PROVER\": \"$CHAIN\"
      }
    }
  }"
```

9. Update verifier set

```bash
axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 1.2
```

### Setup reward pools

| Network              | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
| -------------------- | ---------------- | ------------------------- | ------------------- |
| **Devnet-amplifier** | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Stagenet**         | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Testnet**          | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Mainnet**          | `TBD`            | `TBD`                     | `TBD`               |

10. Create reward pool for voting verifier

```bash
node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in stellar voting verifier" \
  -d "Create pool for stellar in stellar voting verifier" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"100\",
        \"participation_threshold\": [
          \"8\",
          \"10\"
        ],
        \"rewards_per_epoch\": \"100\"
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$VOTING_VERIFIER\"
      }
    }
  }"
```

11. Create reward pool for multisig

```bash
node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in axelar multisig" \
  -d "Create pool for stellar in axelar multisig" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"100\",
        \"participation_threshold\": [
          \"8\",
          \"10\"
        ],
        \"rewards_per_epoch\": \"100\"
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$MULTISIG\"
      }
    }
  }"
```

12. Add funds to reward pools

```bash
axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET

axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET
```

13. Update ampd with the Stellar chain configuration.

```bash
cosmwasm_contract="[$VOTING_VERIFIER\"]"
type="StellarMsgVerifier"
```

## Checklist

The [Stellar GMP checklist](../stellar/2025-01-GMP-v1.0.0.md) will test GMP call.
