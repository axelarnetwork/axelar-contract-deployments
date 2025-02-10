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

### Pre-requisites

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

| Network              | `serviceName` | `votingThreshold` | `signingThreshold` | `confirmationHeight` |
| -------------------- | ------------- | ----------------- | ------------------ | -------------------- |
| **Devnet-amplifier** | `validators`  | `["6", "10"]`     | `["6", "10"]`      | `1`                  |
| **Stagenet**         | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    | `1`                  |
| **Testnet**          | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    | `1`                  |
| **Mainnet**          | `amplifier`   | `["2", "3"]`      | `["2", "3"]`       | `1`                  |

```bash
# Add under `config.axelar.contracts.VotingVerifier` based on Network
\"$CHAIN\" : {
  "governanceAddress": "[governance address]",
  "serviceName": "[service name]",
  "sourceGatewayAddress": "[external gateway address]",
  "votingThreshold": "[voting threshold]",
  "blockExpiry": 10,
  "confirmationHeight": 1,
  "msgIdFormat": "hex_tx_hash_and_event_index",
  "addressFormat": "stellar"
}

# Add under `config.axelar.contracts.MultisigProver` based on Network
\"$CHAIN\" : {
  "governanceAddress": "[governance address]",
  "adminAddress": "[admin address]",
  "signingThreshold": "[signing threshold]",
  "serviceName": "[service name]",
  "verifierSetDiffThreshold": 0,
  "encoder": "stellar_xdr",
  "keyType": "ed25519"
}
```

### Instantiate Amplifier contracts

```bash
CONTRACT_ADMIN=[wasm contract admin address for the upgrade and migration]
```

1. Instantiate `VotingVerifier`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
```

2. Instantiate `Gateway`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
```

3. Instantiate `MultisigProver`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN

```

4. Set environment variables

- Network-specific environment variables: These variables need to be updated by the network.

```bash
VOTING_VERIFIER=$(cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.VotingVerifier[\"$CHAIN\"].address" | tr -d '"')
GATEWAY=$(cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.Gateway[\"$CHAIN\"].address" | tr -d '"')
MULTISIG_PROVER=$(cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.MultisigProver[\"$CHAIN\"].address" | tr -d '"')
MULTISIG=$(cat ./axelar-chains-config/info/$ENV.json | jq .axelar.contracts.Multisig.address | tr -d '"')
REWARDS=$(cat ./axelar-chains-config/info/$ENV.json | jq .axelar.contracts.Rewards.address | tr -d '"')
```

- Gov proposal environment variables. Update these for each network

```bash
RUN_AS_ACCOUNT=[wasm deployer key address]
PROVER_ADMIN=[prover admin]
DEPOSIT_VALUE=100000000
REWARD_AMOUNT=1000000uamplifier
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

6. Update ampd with the Stellar chain configuration. Verifiers should use their own Stellar RPC node for the `http_url` in production.

| Network              | `http_url`                             |
| -------------------- | -------------------------------------- |
| **Devnet-amplifier** | `https://horizon-testnet.stellar.org/` |
| **Stagenet**         | `https://horizon-testnet.stellar.org/` |
| **Testnet**          | `https://horizon-testnet.stellar.org/` |
| **Mainnet**          | `https://horizon.stellar.org`          |

```bash
[[handlers]]
type="StellarMsgVerifier"
http_url=[http url]
cosmwasm_contract="[\"$VOTING_VERIFIER\"]"


[[handlers]]
type="StellarVerifierSetVerifier"
http_url=[http url]
cosmwasm_contract="[\"$VOTING_VERIFIER\"]"

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

### Setup reward pools

9. Create reward pool for voting verifier

#### Voting Verifier Reward

| Network              | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
| -------------------- | ---------------- | ------------------------- | ------------------- |
| **Devnet-amplifier** | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Stagenet**         | `"100"`          | `["7", "10"]`             | `"100"`             |
| **Testnet**          | `"100"`          | `["7", "10"]`             | `"100"`             |
| **Mainnet**          | `TBD`            | `TBD`                     | `TBD`               |

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
        \"epoch_duration\": [epoch duration],
        \"participation_threshold\": [participation threshold],
        \"rewards_per_epoch\": [rewards per epoch]
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$VOTING_VERIFIER\"
      }
    }
  }"
```

10. Create reward pool for multisig

#### MultiSig Reward

| Network              | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
| -------------------- | ---------------- | ------------------------- | ------------------- |
| **Devnet-amplifier** | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Stagenet**         | `"100"`          | `["8", "10"]`             | `"100"`             |
| **Testnet**          | `"100"`          | `["7", "10"]`             | `"100"`             |
| **Mainnet**          | `TBD`            | `TBD`                     | `TBD`               |

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
        \"epoch_duration\": [epoch duration],
        \"participation_threshold\": [participation threshold],
        \"rewards_per_epoch\": [rewards per epoch]
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$MULTISIG\"
      }
    }
  }"
```

11. Add funds to reward pools from a wallet containing the reward funds `$REWARD_AMOUNT`

```bash
axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET

axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET
```

12. Update ampd with the Stellar chain configuration.

```bash
ampd register-public-key ed25519

ampd register-chain-support "[service name]" $CHAIN
```

13. Create genesis verifier set

Note that this step can only be run once a sufficient number of verifiers have registered.

```bash
axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from $PROVER_ADMIN --gas auto --gas-adjustment 1.2
```

## Checklist

The [Stellar GMP checklist](../stellar/2025-01-GMP-v1.0.0.md) will test GMP call.
