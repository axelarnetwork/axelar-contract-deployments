# 1. get config file

wget "https://axelar-devnet.s3.us-east-2.amazonaws.com/devnet-euro-v1/devnet.json" --quiet -O "axelar-contract-deployments/axelar-chains-config/info/devnet1.json"

# 2. get amplifier mnemonic required to deploy wasm contracts

kubectl exec genesis-0 -c core -n devnet-euro-v1 -- sh -c 'cat /home/axelard/.axelar/info/amplifier.txt | tail -1'

# this will return the value of MNEMONIC, required to deploy wasm contracts

# 3. update ENV

MNEMONIC=<from-step-2>
PRIVATE_KEY=<your_private_key>
SIGNATURE_SCHEME=secp256k1
ENV=devnet

# 4. Add sui config

```json
"sui": {
    "name": "Sui",
    "axelarId": "sui",
    "networkType": "testnet",
    "tokenSymbol": "SUI",
    "rpc": "https://fullnode.testnet.sui.io:443",
    "contracts": {}
  }
```

# 5. deploy sui contracts

```sh
node sui/deploy-contract.js deploy Utils -y
node sui/deploy-contract.js deploy VersionControl -y
node sui/deploy-contract.js deploy AxelarGateway --domainSeparator offline --minimumRotationDelay 0 --signers wallet -y
node sui/deploy-contract.js deploy RelayerDiscovery -y
node sui/deploy-contract.js deploy GasService -y
node sui/deploy-contract.js deploy Abi -y
node sui/deploy-contract.js deploy ITS -y
node sui/deploy-contract.js deploy Example -y
```

# 6. Update devnet.json chains:

```json
 "sui": {
        "axelarId": "sui",
        "id": "sui"
      }
```

```sh
CHAIN=sui
CHAIN_ID=1
NAMESPACE=devnet-euro-v1
SOURCE_GATEWAY_ADDRESS=0xe66e69068b64bd0efd9db5f9e33836535186a8877b499dcbab86ddb7b0b6353f
ENCODER=bcs
MSG_ID_FORMAT=base58_tx_digest_and_event_index
ADDRESS_FORMAT=sui
```

# 7. deploy wasm contracts

```sh
# deploy voting verifier
node cosmwasm/deploy-contract.js \
	-c VotingVerifier \
	-n sui \
	-s sui \
	--instantiate2 --reuseCodeId -y

# Deploy Gateway
node cosmwasm/deploy-contract.js \
	-c Gateway \
	-n sui \
	-s sui \
	--instantiate2 --reuseCodeId -y

# Deploy MultisigProver
node cosmwasm/deploy-contract.js \
	-c MultisigProver \
	-n sui \
	-s sui \
	--instantiate2 --reuseCodeId -y
```

# 8. create pools (usually run via governance)

```sh
kubectl exec -ti genesis-0 -c core -n devnet-euro-v1 -- sh

# For rewards v1.1 and later: create rewards pool for verifying
# rewards_per_epoch needs to be confirmed with @canh
axelard tx wasm execute <rewards-address> '{"create_pool":{"params":{"epoch_duration":"100" , "participation_threshold":["8","10"],"rewards_per_epoch":"100"},"pool_id":{"chain_name":"<chain-name>","contract":"<voting verifier address>"}}}' --from amplifier --gas auto --gas-adjustment 2

axelard tx wasm execute axelar1wkwy0xh89ksdgj9hr347dyd2dw7zesmtrue6kfzyml4vdtz6e5ws2pvc5e '{"create_pool":{"params":{"epoch_duration":"100" , "participation_threshold":["8","10"],"rewards_per_epoch":"100"},"pool_id":{"chain_name":"sui","contract":"axelar1466nf3zuxpya8q9emxukd7vftaf6h4psr0a07srl5zw74zh84yjq4687qd"}}}' --from amplifier --gas auto --gas-adjustment 2

# For rewards v1.1 and later: create rewards pool for signing
axelard tx wasm execute <rewards-address> '{"create_pool":{"params":{"epoch_duration":"100", "participation_threshold":["8","10"],"rewards_per_epoch":"100"},"pool_id":{"chain_name":"<chain-name>","contract":"<multisig address>"}}}' --from amplifier --gas auto --gas-adjustment 2

axelard tx wasm execute axelar1wkwy0xh89ksdgj9hr347dyd2dw7zesmtrue6kfzyml4vdtz6e5ws2pvc5e '{"create_pool":{"params":{"epoch_duration":"100" , "participation_threshold":["8","10"],"rewards_per_epoch":"100"},"pool_id":{"chain_name":"sui","contract":"axelar1nupsqyy8lh85h5n56t5j9e6anc9n2rnccwqk3p386uqsdzxvjf3swmsjyf"}}}' --from amplifier --gas auto --gas-adjustment 2

echo $KEYRING_PASSWORD | axelard tx wasm execute <rewards-address> '{"add_rewards":{"pool_id":{"chain_name":"<chain-name>","contract":"<multisig address>"}}}' --amount 100000000uaxl --from validator

axelard tx wasm execute axelar1wkwy0xh89ksdgj9hr347dyd2dw7zesmtrue6kfzyml4vdtz6e5ws2pvc5e '{"add_rewards":{"pool_id":{"chain_name":"sui","contract":"axelar1466nf3zuxpya8q9emxukd7vftaf6h4psr0a07srl5zw74zh84yjq4687qd"}}}' --amount 100000000uayushsui --from validator

echo $KEYRING_PASSWORD | axelard tx wasm execute <rewards-address> '{"add_rewards":{"pool_id":{"chain_name":"<chain-name>","contract":"<voting verifier address>"}}}' --amount 100000000uaxl --from validator

axelard tx wasm execute axelar1wkwy0xh89ksdgj9hr347dyd2dw7zesmtrue6kfzyml4vdtz6e5ws2pvc5e '{"add_rewards":{"pool_id":{"chain_name":"sui","contract":"axelar1nupsqyy8lh85h5n56t5j9e6anc9n2rnccwqk3p386uqsdzxvjf3swmsjyf"}}}' --amount 100000000uayushsui --from validator
```

# 9. Register Gateway address at Router contract

```sh
JSON_CMD_REGISTER="{\"register_chain\": {\"chain\": \"sui\",\"gateway_address\": \"axelar1rxpyp5ns6ydtefs3y4dx8l77ulj8kalfrtpx9a9s9ly4uue42l4sk6r3x6\",\"msg_id_format\":\"base58_tx_digest_and_event_index\"}}"

axelard tx wasm execute "axelar163vlykge6yz2jpy4j6requdugq2ly209wgt7l3a9dadut2jw6zzqlfczpu" "$JSON_CMD_REGISTER" --from amplifier --gas auto --gas-adjustment 2
```

# 10. Authorize Multisig Prover address at Multisig contract

```sh
JSON="{\"authorize_callers\":{\"contracts\":{\"axelar1fwc9rue2z77zaqten8rqd4uxqcaq8tn73xsfflfm89tl76k86gsspkz4nv\":\"sui\"}}}"

axelard tx wasm execute "axelar1466nf3zuxpya8q9emxukd7vftaf6h4psr0a07srl5zw74zh84yjq4687qd" "$JSON" --from amplifier --gas auto --gas-adjustment 2
```

# 11. Register Multisig Prover address at Coordinator contract

```sh
JSON="{\"register_prover_contract\":{\"chain_name\":\"sui\",\"new_prover_addr\":\"axelar1fxqqhv9ushagx3903pfegpz0m369cr5fwajpm4yhlphl8anjzduqvaqzlw\"}}"


axelard tx wasm execute "axelar1ufs3tlq4umljk0qfe8k5ya0x6hpavn897u2cnf9k0en9jr7qarqqa9263g" "$JSON" --from amplifier --gas auto --gas-adjustment 2
```

# 12. update CM and register chain support

```sh
KUBE_EDITOR="nvim" kubectl edit cm/ampd-set-1-axelar-amplifier-worker-config -n devnet-euro-v1

    [[handlers]]
    cosmwasm_contract="axelar1qum2tr7hh4y7ruzew68c64myjec0dq2s2njf6waja5t0w879lutqv062tl"
    rpc_url="https://fullnode.testnet.sui.io:443"
    type="SuiMsgVerifier"

    [[handlers]]
    cosmwasm_contract="axelar1qum2tr7hh4y7ruzew68c64myjec0dq2s2njf6waja5t0w879lutqv062tl"
    rpc_url="https://fullnode.testnet.sui.io:443"
    type="SuiVerifierSetVerifier"

kubectl rollout restart sts ampd-set-1-axelar-amplifier-worker -n devnet-euro-v1
kubectl get pods -n devnet-euro-v1 --watch

kubectl exec -ti ampd-set-1-axelar-amplifier-worker-0 -c ampd-verifier-address -n devnet-euro-v1 -- sh
ampd register-chain-support validators sui

kubectl exec -ti ampd-set-1-axelar-amplifier-worker-1 -c ampd-verifier-address -n devnet-euro-v1 -- sh
ampd register-chain-support validators sui

kubectl exec -ti ampd-set-1-axelar-amplifier-worker-2 -c ampd-verifier-address -n devnet-euro-v1 -- sh
ampd register-chain-support validators sui
```

13. Update verifier set

```sh
kubectl exec -ti genesis-0 -c core -n devnet-euro-v1 -- sh

axelard tx wasm execute axelar1fxqqhv9ushagx3903pfegpz0m369cr5fwajpm4yhlphl8anjzduqvaqzlw '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 2
```

14. Rotate signers

```
node sui/gateway.js rotate --proof wallet
```

15. Update config in s3

```
aws s3 cp ./axelar-chains-config/info/devnet.json "s3://axelar-devnet/devnet-euro-v1/devnet.json" --acl public-read
```

16. e2e tests
