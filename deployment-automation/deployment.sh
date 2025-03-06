#!/bin/bash

echo "🚀 Welcome to Axelar Deployment Setup 🚀"


# Function to validate network input
get_network_name() {
    echo "Select a network:"
    echo "1) mainnet"
    echo "2) testnet"
    echo "3) stagenet"
    echo "4) devnet-amplifier"
    echo "5) Custom devnet (e.g., devnet-user)"
    
    while true; do
        read -p "Enter your choice (1-5): " choice
        case $choice in
            1) NAMESPACE="mainnet"; break ;;
            2) NAMESPACE="testnet"; break ;;
            3) NAMESPACE="stagenet"; break ;;
            4) NAMESPACE="devnet-amplifier"; break ;;
            5) read -p "Enter your custom devnet name (e.g., devnet-user): " custom_name
               NAMESPACE="$custom_name"
               break ;;
            *) echo "❌ Invalid choice. Please select 1, 2, 3, 4 or 5." ;;
        esac
    done
}

# Function to check if the network is a custom devnet
is_custom_devnet() {
    if [[ "$NAMESPACE" == "mainnet" || "$NAMESPACE" == "testnet" || "$NAMESPACE" == "stagenet" || "$NAMESPACE" == "devnet-amplifier" ]]; then
        return 1  # Not a custom devnet
    else
        return 0  # Is a custom devnet
    fi
}


# Function to set predefined values for known networks
set_predefined_values() {
    case $NAMESPACE in
        "mainnet")
            GOVERNANCE_ADDRESS="axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"
            ADMIN_ADDRESS="axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj"
            SERVICE_NAME="amplifier"
            VOTING_THRESHOLD='["2", "3"]'
            SIGNING_THRESHOLD='["2", "3"]'
            CONFIRMATION_HEIGHT="1"
            MINIMUM_ROTATION_DELAY="86400"
            DEPLOYMENT_TYPE="create"
            DEPLOYER="0xB8Cd93C83A974649D76B1c19f311f639e62272BC"
            CONTRACT_ADMIN="axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am"
            PROVER_ADMIN="axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj"
            DEPOSIT_VALUE="2000000000"
            REWARD_AMOUNT="1000000uaxl"
            ;;
        "testnet")
            GOVERNANCE_ADDRESS="axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"
            ADMIN_ADDRESS="axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35"
            SERVICE_NAME="amplifier"
            VOTING_THRESHOLD='["51", "100"]'
            SIGNING_THRESHOLD='["51", "100"]'
            CONFIRMATION_HEIGHT="1"
            MINIMUM_ROTATION_DELAY="3600"
            DEPLOYMENT_TYPE="create"
            DEPLOYER="0xba76c6980428A0b10CFC5d8ccb61949677A61233"
            CONTRACT_ADMIN="axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7"
            PROVER_ADMIN="axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35"
            DEPOSIT_VALUE="2000000000"
            REWARD_AMOUNT="1000000uaxl"
            ;;
        "stagenet")
            GOVERNANCE_ADDRESS="axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"
            ADMIN_ADDRESS="axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv"
            SERVICE_NAME="amplifier"
            VOTING_THRESHOLD='["51", "100"]'
            SIGNING_THRESHOLD='["51", "100"]'
            CONFIRMATION_HEIGHT="1"
            MINIMUM_ROTATION_DELAY="300"
            DEPLOYMENT_TYPE="create3"
            DEPLOYER="0xba76c6980428A0b10CFC5d8ccb61949677A61233"
            CONTRACT_ADMIN="axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky"
            PROVER_ADMIN="axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv"
            DEPOSIT_VALUE="100000000"
            REWARD_AMOUNT="1000000uaxl"
            ;;
        "devnet-amplifier")
            GOVERNANCE_ADDRESS="axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"
            ADMIN_ADDRESS="axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"
            SERVICE_NAME="validators"
            VOTING_THRESHOLD='["6", "10"]'
            SIGNING_THRESHOLD='["6", "10"]'
            CONFIRMATION_HEIGHT="1"
            MINIMUM_ROTATION_DELAY="0"
            DEPLOYMENT_TYPE="create3"
            DEPLOYER="0xba76c6980428A0b10CFC5d8ccb61949677A61233"
            CONTRACT_ADMIN="axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"
            PROVER_ADMIN="axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"
            DEPOSIT_VALUE="100000000"
            REWARD_AMOUNT="1000000uamplifier"
            ;;
    esac

    # Export the values as environment variables for use in other functions
    export GOVERNANCE_ADDRESS
    export ADMIN_ADDRESS
    export SERVICE_NAME
    export VOTING_THRESHOLD
    export SIGNING_THRESHOLD
    export CONFIRMATION_HEIGHT
    export MINIMUM_ROTATION_DELAY
    export DEPLOYMENT_TYPE
    export DEPLOYER
    export CONTRACT_ADMIN
    export PROVER_ADMIN
    export DEPOSIT_VALUE
    export REWARD_AMOUNT
}


# Function to validate private key
validate_private_key() {
    while true; do
        read -p "Enter Private Key (must start with 0x): " TARGET_CHAIN_PRIVATE_KEY
        if [[ "$TARGET_CHAIN_PRIVATE_KEY" =~ ^0x[0-9a-fA-F]+$ ]]; then
            break
        else
            echo "❌ Invalid private key format. Make sure it starts with '0x' and contains only hexadecimal characters (0-9, a-f)."
        fi
    done
}

# Function to validate RPC URL
validate_rpc_url() {
    while true; do
        read -p "Enter RPC URL (must start with http:// or https://): " RPC_URL
        if [[ "$RPC_URL" =~ ^https?:// ]]; then
            break
        else
            echo "❌ Invalid RPC URL format. It must start with 'http://' or 'https://'."
        fi
    done
}

# Function to validate Axelar RPC Node URL
validate_axelar_rpc_url() {
    while true; do
        read -p "Enter Axelar RPC Node URL (must start with http:// or https://): " AXELAR_RPC_URL
        if [[ "$AXELAR_RPC_URL" =~ ^https?:// ]]; then
            break
        else
            echo "❌ Invalid Axelar RPC Node URL format. It must start with 'http://' or 'https://'."
        fi
    done
}

# Function to create a wallet using the provided MNEMONIC
create_wallet() {
    local wallet_name="amplifier"

    echo "⚡ Creating wallet '$wallet_name' using provided MNEMONIC..."

    # Check if the wallet already exists
    existing_wallet=$(axelard keys show "$wallet_name" --keyring-backend test 2>/dev/null)
    
    if [[ -n "$existing_wallet" ]]; then
        echo "⚠️ Wallet '$wallet_name' already exists. Overwriting..."
    fi

    # Automate the wallet recovery process
    echo -e "y\n$MNEMONIC" | axelard keys add --keyring-backend test --recover "$wallet_name"

    # Verify wallet creation
    wallet_address=$(axelard keys show "$wallet_name" --keyring-backend test -a)

    if [[ -n "$wallet_address" ]]; then
        echo "✅ Wallet successfully created! Address: $wallet_address"
        export WALLET_ADDRESS="$wallet_address"
    else
        echo "❌ Failed to create wallet!"
        exit 1
    fi
}

# Function to extract the Predicted Gateway Proxy Address
extract_proxy_gateway_address() {
    local extracted_address=$(echo "$1" | awk '/Predicted gateway proxy address:/ {print $NF}')

    if [[ -n "$extracted_address" && "$extracted_address" =~ ^0x[a-fA-F0-9]+$ ]]; then
        export PROXY_GATEWAY_ADDRESS="$extracted_address"
        echo "✅ Extracted and set PROXY_GATEWAY_ADDRESS: $PROXY_GATEWAY_ADDRESS"
    else
        echo "❌ Could not extract Predicted Gateway Proxy Address!"
        exit 1
    fi
}

# Function to generate the JSON config file
generate_json_config() {
    local json_content=$(cat <<EOF
{
    "$CHAIN_NAME": {
        "name": "$CHAIN_NAME",
        "id": "$CHAIN_ID",
        "axelarId": "$CHAIN_NAME",
        "chainId": $CHAIN_ID,
        "rpc": "$RPC_URL",
        "tokenSymbol": "$TOKEN_SYMBOL",
        "confirmations": 1,
        "gasOptions": {
            "gasLimit": $GAS_LIMIT
        }
    }
}
EOF
)
    echo "$json_content" > "./config.json"
    echo "✅ Configuration saved to ./config.json"
}

# Function to insert the generated JSON into the network config file
insert_into_network_config() {
    local network_json_path="../axelar-chains-config/info/$NAMESPACE.json"

    # Check if the network JSON file exists
    if [[ ! -f "$network_json_path" ]]; then
        echo "❌ Network JSON file not found: $network_json_path"
        exit 1
    fi

    # Read the JSON file
    local existing_json=$(cat "$network_json_path")

    # Check if "chains" exists in the JSON
    if ! echo "$existing_json" | jq -e '.chains' >/dev/null; then
        echo "❌ No 'chains' dictionary found in $network_json_path"
        exit 1
    fi

    # Check if CHAIN_NAME already exists in "chains"
    if echo "$existing_json" | jq -e --arg chain "$CHAIN_NAME" '.chains[$chain]' >/dev/null; then
        echo "❌ Chain '$CHAIN_NAME' already exists in $network_json_path! Aborting to prevent overwriting."
        exit 1
    fi

    # Insert the new chain object into "chains"
    local updated_json=$(echo "$existing_json" | jq --argjson newChain "$(cat ./config.json)" '.chains += $newChain')

    # Write back the updated JSON
    echo "$updated_json" > "$network_json_path"
    echo "✅ Successfully added '$CHAIN_NAME' to $network_json_path"
}

# Function to update VotingVerifier inside axelar.contracts in JSON config
update_voting_verifier_config() {
    local network_json_path="../axelar-chains-config/info/$NAMESPACE.json"

    # Ensure the JSON file exists
    if [[ ! -f "$network_json_path" ]]; then
        echo "❌ Network JSON file not found: $network_json_path"
        exit 1
    fi

    # Read the existing JSON file
    local existing_json=$(cat "$network_json_path")

    # Check if "axelar.contracts.VotingVerifier" exists in the JSON
    if ! echo "$existing_json" | jq -e '.axelar.contracts.VotingVerifier' >/dev/null; then
        echo "❌ No 'VotingVerifier' section found inside axelar.contracts in $network_json_path!"
        exit 1
    fi

    # Check if CHAIN_NAME already exists in VotingVerifier
    if echo "$existing_json" | jq -e --arg chain "$CHAIN_NAME" '.axelar.contracts.VotingVerifier[$chain]' >/dev/null; then
        echo "❌ Chain '$CHAIN_NAME' already exists in VotingVerifier! Aborting to prevent overwriting."
        exit 1
    fi

    # Create the new chain entry
    local new_chain_entry=$(jq -n \
        --arg governanceAddress "$GOVERNANCE_ADDRESS" \
        --arg sourceGatewayAddress "$PROXY_GATEWAY_ADDRESS" \
        --arg serviceName "$SERVICE_NAME" \
        --argjson votingThreshold "$VOTING_THRESHOLD" \
        --argjson confirmationHeight "$CONFIRMATION_HEIGHT" \
        '{
            governanceAddress: $governanceAddress,
            serviceName: $serviceName,
            sourceGatewayAddress: $sourceGatewayAddress,
            votingThreshold: $votingThreshold,
            blockExpiry: 10,
            confirmationHeight: $confirmationHeight,
            msgIdFormat: "hex_tx_hash_and_event_index",
            addressFormat: "eip55"
        }')

    # Insert the new chain entry into axelar.contracts.VotingVerifier
    local updated_json=$(echo "$existing_json" | jq --argjson newChain "$new_chain_entry" --arg chain "$CHAIN_NAME" '.axelar.contracts.VotingVerifier[$chain] = $newChain')

    # Write the updated JSON back to file
    echo "$updated_json" > "$network_json_path"
    echo "✅ Successfully added '$CHAIN_NAME' to VotingVerifier inside axelar.contracts in $network_json_path"
}

# Function to update the namespace JSON file with MultisigProver contract
update_multisig_prover() {
    local namespace_json_path="../axelar-chains-config/info/$NAMESPACE.json"

    # Check if the namespace JSON file exists
    if [[ ! -f "$namespace_json_path" ]]; then
        echo "❌ Namespace JSON file not found: $namespace_json_path"
        exit 1
    fi

    # Read the existing JSON file
    local existing_json=$(cat "$namespace_json_path")

    # Check if "axelar.contracts.MultisigProver" exists in the JSON
    if ! echo "$existing_json" | jq -e '.axelar.contracts.MultisigProver' >/dev/null; then
        echo "❌ No 'MultisigProver' dictionary found in $namespace_json_path"
        exit 1
    fi

    # Check if CHAIN_NAME already exists in "MultisigProver"
    if echo "$existing_json" | jq -e --arg chain "$CHAIN_NAME" '.axelar.contracts.MultisigProver[$chain]' >/dev/null; then
        echo "❌ Chain '$CHAIN_NAME' already exists under 'MultisigProver' in $namespace_json_path! Aborting to prevent overwriting."
        exit 1
    fi

    # Create the new chain entry with updated environment variables
    local new_multisig_prover_entry=$(jq -n \
        --arg governanceAddress "$GOVERNANCE_ADDRESS" \
        --arg adminAddress "$ADMIN_ADDRESS" \
        --arg destinationChainID "$CHAIN_ID" \
        --arg serviceName "$SERVICE_NAME" \
        --argjson signingThreshold "$SIGNING_THRESHOLD" \
        '{
            "governanceAddress": $governanceAddress,
            "adminAddress": $adminAddress,
            "destinationChainID": $destinationChainID,
            "signingThreshold": $signingThreshold,
            "serviceName": $serviceName,
            "verifierSetDiffThreshold": 0,
            "encoder": "abi",
            "keyType": "ecdsa"
        }')

    # Insert the new chain entry into "MultisigProver"
    local updated_json=$(echo "$existing_json" | jq --arg chain "$CHAIN_NAME" --argjson newEntry "$new_multisig_prover_entry" \
        '.axelar.contracts.MultisigProver[$chain] = $newEntry')

    # Write back the updated JSON
    echo "$updated_json" > "$namespace_json_path"
    echo "✅ Successfully added '$CHAIN_NAME' under 'MultisigProver' in $namespace_json_path"

    # Confirm the new entry was added
    echo "🔍 Verifying the new MultisigProver entry..."
    jq '.axelar.contracts.MultisigProver' "$namespace_json_path"
}




# Function to extract SALT value from the correct checksums file
extract_salt() {
    local contract_name="$1"  # Contract name, e.g., "voting-verifier"
    local checksum_file="../wasm/${contract_name}_checksums.txt"

    if [[ ! -f "$checksum_file" ]]; then
        echo "❌ Checksum file not found: $checksum_file"
        exit 1
    fi

    # Extract the correct checksum (SALT) for the contract
    local salt_value=$(grep "${contract_name}.wasm" "$checksum_file" | awk '{print $1;}')

    if [[ -z "$salt_value" ]]; then
        echo "❌ Failed to extract SALT for $contract_name!"
        exit 1
    fi

    export SALT="$salt_value"
    echo "✅ Extracted SALT: $SALT"
}

# Extract ROUTER_ADDRESS from the namespace JSON file
extract_router_address() {
    local router_file="../axelar-chains-config/info/$NAMESPACE.json"

    if [[ ! -f "$router_file" ]]; then
        echo "❌ Router config file not found: $router_file"
        exit 1
    fi

    ROUTER_ADDRESS=$(jq -rM '.axelar.contracts.Router.address' "$router_file")
    
    if [[ -z "$ROUTER_ADDRESS" || "$ROUTER_ADDRESS" == "null" ]]; then
        echo "❌ Could not extract ROUTER_ADDRESS!"
        exit 1
    fi

    export ROUTER_ADDRESS
    echo "✅ Extracted ROUTER_ADDRESS: $ROUTER_ADDRESS"
}

# Extract GATEWAY_ADDRESS for the specified chain
extract_gateway_address() {
    local gateway_file="../axelar-chains-config/info/$NAMESPACE.json"
    local query=".axelar.contracts.Gateway.${CHAIN_NAME}.address"

    if [[ ! -f "$gateway_file" ]]; then
        echo "❌ Gateway config file not found: $gateway_file"
        exit 1
    fi

    GATEWAY_ADDRESS=$(jq -rM "$query" "$gateway_file")

    if [[ -z "$GATEWAY_ADDRESS" || "$GATEWAY_ADDRESS" == "null" ]]; then
        echo "❌ Could not extract GATEWAY_ADDRESS for $CHAIN_NAME!"
        exit 1
    fi

    export GATEWAY_ADDRESS
    echo "✅ Extracted GATEWAY_ADDRESS: $GATEWAY_ADDRESS"
}

# Function to build JSON command for chain registration
build_json_cmd_register() {
    JSON_CMD_REGISTER="{\"register_chain\": {\"chain\": \"$CHAIN_NAME\", \"gateway_address\": \"$GATEWAY_ADDRESS\", \"msg_id_format\":\"hex_tx_hash_and_event_index\"}}"
    echo "✅ Built JSON_CMD_REGISTER: $JSON_CMD_REGISTER"
}

# Function to verify the transaction execution
verify_execution() {
    echo "⚡ Verifying the transaction execution..."

    JSON_QUERY="{\"chain_info\": \"$CHAIN_NAME\"}"

    verification_output=$(axelard q wasm contract-state smart "$ROUTER_ADDRESS" "$JSON_QUERY" --node "$AXELAR_RPC_URL" 2>&1)

    # Print raw output for debugging
    echo "🔍 Verification Output:"
    echo "$verification_output"

    # Extract Gateway Address
    VERIFIED_GATEWAY_ADDRESS=$(echo "$verification_output" | awk '/gateway:/ {getline; print $2}' | tr -d ' ')

    # Ensure the gateway address matches expected value
    if [[ -n "$VERIFIED_GATEWAY_ADDRESS" && "$VERIFIED_GATEWAY_ADDRESS" == "$GATEWAY_ADDRESS" ]]; then
        echo "✅ Verification successful! Gateway address matches: $VERIFIED_GATEWAY_ADDRESS"
    else
        echo "❌ Verification failed! Expected: $GATEWAY_ADDRESS, Got: $VERIFIED_GATEWAY_ADDRESS"
        exit 1
    fi
}

verify_multisig() {
    echo "⚡ Verifying the transaction execution for MultisigProver..."

    JSON_QUERY="{\"is_caller_authorized\": {\"contract_address\": \"$MULTISIG_PROVER_ADDRESS\", \"chain_name\": \"$CHAIN_NAME\"}}"

    verification_output=$(axelard q wasm contract-state smart "$MULTISIG_ADDRESS" "$JSON_QUERY" --node "$AXELAR_RPC_URL" 2>&1)

    # Print raw output for debugging
    echo "🔍 Verification Output:"
    echo "$verification_output"

    # Check if the output contains "data: true" as plain text
    if echo "$verification_output" | grep -q "data: true"; then
        echo "✅ Verification successful! MultisigProver is authorized."
    else
        echo "❌ Verification failed! Expected 'data: true' but got:"
        echo "$verification_output"
        exit 1
    fi
}


create_reward_pools() {
    echo "⚡ Creating reward pools"
    if is_custom_devnet; then
        PARAMS="{\"epoch_duration\": \"10\",\"rewards_per_epoch\": \"100\",\"participation_threshold\": [\"9\",\"10\"]}"
        JSON_CREATE_POOL_MULTISIG="{\"create_pool\":{\"pool_id\":{\"chain_name\":\"$CHAIN_NAME\",\"contract\":\"$MULTISIG_ADDRESS\"},\"params\":$PARAMS}}"
        JSON_CREATE_POOL_VERIFIER="{\"create_pool\":{\"pool_id\":{\"chain_name\":\"$CHAIN_NAME\",\"contract\":\"$VOTING_VERIFIER_ADDRESS\"},\"params\":$PARAMS}}"


        axelard tx wasm execute "$REWARDS_ADDRESS" "$JSON_CREATE_POOL_MULTISIG" \
            --from amplifier \
            --gas auto \
            --gas-adjustment 2 \
            --node "$AXELAR_RPC_URL" \
            --gas-prices 0.00005"$TOKEN_DENOM" \
            --keyring-backend test \
            --chain-id "$NAMESPACE"

        axelard tx wasm execute "$REWARDS_ADDRESS" "$JSON_CREATE_POOL_VERIFIER" \
            --from amplifier \
            --gas auto \
            --gas-adjustment 2 \
            --node "$AXELAR_RPC_URL" \
            --gas-prices 0.00005"$TOKEN_DENOM" \
            --keyring-backend test \
            --chain-id "$NAMESPACE"
    else
        if $NAMESPACE = "devnet-amplifier"; then
            node ../cosmwasm/submit-proposal.js execute \
                -c Rewards \
                -t "Create pool for $CHAIN in $CHAIN voting verifier" \
                -d "Create pool for $CHAIN in $CHAIN voting verifier" \
                --runAs $RUN_AS_ACCOUNT \
                --deposit $DEPOSIT_VALUE \
                --msg "{ \"create_pool\": { \"params\": { \"epoch_duration\": \"$EPOCH_DURATION\", \"participation_threshold\": [\"7\", \"10\"], \"rewards_per_epoch\": \"100\" }, \"pool_id\": { \"chain_name\": \"$CHAIN_NAME\", \"contract\": \"$VOTING_VERIFIER\" } } }"
        else
            node ../cosmwasm/submit-proposal.js execute \
                -c Rewards \
                -t "Create pool for $CHAIN in $CHAIN voting verifier" \
                -d "Create pool for $CHAIN in $CHAIN voting verifier" \
                --deposit $DEPOSIT_VALUE \
                --msg "{ \"create_pool\": { \"params\": { \"epoch_duration\": \"$EPOCH_DURATION\", \"participation_threshold\": [\"7\", \"10\"], \"rewards_per_epoch\": \"100\" }, \"pool_id\": { \"chain_name\": \"$CHAIN_NAME\", \"contract\": \"$VOTING_VERIFIER\" } } }"
        fi
    fi

}


add_funds_to_pools() {
    if ! is_custom_devnet; then
        echo "⚡ Adding funds to reward pools..."
        REWARDS=$(cat ../axelar-chains-config/info/"$NAMESPACE".json | jq .axelar.contracts.Rewards.address | tr -d '"')
        axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN_NAME\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET
        axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN_NAME\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET  
    fi
}

create_genesis_verifier_set() {

    axelard tx wasm execute $MULTISIG_PROVER_ADDRESS '"update_verifier_set"' \
            --from $PROVER_ADMIN \
            --gas auto \
            --gas-adjustment 2 \
            --node "$AXELAR_RPC_URL" \
            --gas-prices 0.00005"$TOKEN_DENOM" \
            --keyring-backend test \
            --chain-id "$NAMESPACE"
    
    echo "🔍 Querying multisig prover for active verifier set..."
    axelard q wasm contract-state smart $MULTISIG_PROVER_ADDRESS "\"current_verifier_set\"" \
            --node "$AXELAR_RPC_URL" \
            --chain-id "$NAMESPACE"
}

# Function to print environment variables as JSON and exit
print_env_json_and_exit() {
    echo "🎉 Chain registration complete! Need to Update the Verifiers!"
    
    env | grep -E "^(NAMESPACE|CHAIN_NAME|CHAIN_ID|TOKEN_SYMBOL|GAS_LIMIT|TARGET_CHAIN_PRIVATE_KEY|RPC_URL|AXELAR_RPC_URL|MNEMONIC|GOVERNANCE_ADDRESS|ADMIN_ADDRESS|SERVICE_NAME|VOTING_THRESHOLD|SIGNING_THRESHOLD|CONFIRMATION_HEIGHT|MINIMUM_ROTATION_DELAY|DEPLOYMENT_TYPE|DEPLOYER|CONTRACT_ADMIN|PROVER_ADMIN|DEPOSIT_VALUE|REWARD_AMOUNT|TOKEN_DENOM|MULTISIG_ADDRESS|VOTING_VERIFIER_ADDRESS|REWARDS_ADDRESS|ROUTER_ADDRESS|GATEWAY_ADDRESS|MULTISIG_ADDRESS|MULTISIG_PROVER_ADDRESS|COORDINATOR_ADDRESS)=" \
        | awk -F '=' '{gsub(/"/, "\\\"", $2); printf "  \"%s\": \"%s\",\n", $1, $2}' \
        | sed '$ s/,$//' \
        | awk 'BEGIN {print "{"} {print} END {print "}"}' \
        | tee deployment_config.json

    echo "✅ JSON configuration saved to deployment_config.json. You can use it for resuming deployment."
    exit 0
}

deploy_gateway_contract() {

    setup_output=$(node ../evm/deploy-amplifier-gateway.js --env "$NAMESPACE" -n "$CHAIN_NAME" -m "$DEPLOYMENT_TYPE" --minimumRotationDelay "$MINIMUM_ROTATION_DELAY" -p "$TARGET_CHAIN_PRIVATE_KEY" 2>&1)

    # Print output for debugging
    echo "$setup_output"

}

# This is the continuation point if the script is resumed from JSON
goto_after_chain_registration() {
    echo "✅ Continuing deployment from saved state..."

    # Run the verification step that gateway router was registered
    verify_execution

    # Retrieve the Multisig Contract Address
    MULTISIG_ADDRESS=$(cat ../axelar-chains-config/info/"$NAMESPACE".json | jq -rM '.axelar.contracts.Multisig.address')
    export MULTISIG_ADDRESS
    echo "✅ Retrieved MULTISIG_ADDRESS: $MULTISIG_ADDRESS"

    # Retrieve the Multisig Prover Contract Address
    QUERY=".axelar.contracts.MultisigProver.${CHAIN_NAME}.address"
    MULTISIG_PROVER_ADDRESS=$(cat ../axelar-chains-config/info/"$NAMESPACE".json | jq -rM "$QUERY")
    export MULTISIG_PROVER_ADDRESS
    echo "✅ Retrieved MULTISIG_PROVER_ADDRESS: $MULTISIG_PROVER_ADDRESS"

    # Construct JSON Payload for the Execute Call
    JSON_CMD_MULTISIG="{\"authorize_callers\":{\"contracts\":{\"$MULTISIG_PROVER_ADDRESS\":\"$CHAIN_NAME\"}}}"
    echo "📜 JSON Command: $JSON_CMD_MULTISIG"

    COORDINATOR_ADDRESS=$(cat ../axelar-chains-config/info/"$NAMESPACE".json | jq -rM '.axelar.contracts.Coordinator.address')
    export COORDINATOR_ADDRESS
    echo $COORDINATOR_ADDRESS;

    JSON_CMD_MULTISIG_PROVER="{\"register_prover_contract\":{\"chain_name\":\"$CHAIN_NAME\",\"new_prover_addr\":\"$MULTISIG_PROVER_ADDRESS\"}}" 
    echo $JSON_CMD_MULTISIG_PROVER;

    if is_custom_devnet; then
        echo "Register prover contract"

        axelard tx wasm execute "$COORDINATOR_ADDRESS" "$JSON_CMD_MULTISIG_PROVER" \
            --from amplifier \
            --gas auto \
            --gas-adjustment 2 \
            --node "$AXELAR_RPC_URL" \
            --gas-prices 0.00005"$TOKEN_DENOM" \
            --keyring-backend test \
            --chain-id "$NAMESPACE"

        # Execute the Transaction using `kubectl`
        echo "⚡ Executing authorize_callers for Multisig Contract..."

        axelard tx wasm execute "$MULTISIG_ADDRESS" "$JSON_CMD_MULTISIG" \
            --from amplifier \
            --gas auto \
            --gas-adjustment 2 \
            --node "$AXELAR_RPC_URL" \
            --gas-prices 0.00005"$TOKEN_DENOM" \
            --keyring-backend test \
            --chain-id "$NAMESPACE"


    else
        # ACtual networks require proposal for chain integration
        if $NAMESPACE = "devnet-amplifier"; then
            node ../cosmwasm/submit-proposal.js execute \
                -c Coordinator \
                -t "Register Multisig Prover for $CHAIN_NAME" \
                -d "Register Multisig Prover address for $CHAIN_NAME at Coordinator contract" \
                --runAs $RUN_AS_ACCOUNT \
                --deposit $DEPOSIT_VALUE \
                --msg "$JSON_CMD_MULTISIG_PROVER"

            
            node ../cosmwasm/submit-proposal.js execute \
                -c Multisig \
                -t "Authorize Multisig Prover for $CHAIN" \
                -d "Authorize Multisig Prover address for $CHAIN at Multisig contract" \
                --runAs $RUN_AS_ACCOUNT \
                --deposit $DEPOSIT_VALUE \
                --msg "$JSON_CMD_MULTISIG"
        else
            node ../cosmwasm/submit-proposal.js execute \
            -c Coordinator \
            -t "Register Multisig Prover for $CHAIN" \
            -d "Register Multisig Prover address for $CHAIN at Coordinator contract" \
            --deposit $DEPOSIT_VALUE \
            --msg "$JSON_CMD_MULTISIG_PROVER"

            node ../cosmwasm/submit-proposal.js execute \
                -c Multisig \
                -t "Authorize Multisig Prover for $CHAIN" \
                -d "Authorize Multisig Prover address for $CHAIN at Multisig contract" \
                --deposit $DEPOSIT_VALUE \
                --msg "$JSON_CMD_MULTISIG"
        fi

    fi
    
    print_env_json_and_exit

    echo "🔍 Wait for multisig proposals to be approved..."
}


goto_after_multisig_proposals() {
    verify_multisig

    create_reward_pools
    add_funds_to_pools

    create_genesis_verifier_set

    deploy_gateway_contract


    echo "🎉 Deployment complete!"
}
# Ensure WASM directory is correct
WASM_DIR="../wasm"


# Ask user if this is a new deployment or continuation
read -p "Is this a new deployment? (yes/no): " DEPLOYMENT_TYPE

# Check if deployment is a continuation
if [[ "$DEPLOYMENT_TYPE" == "no" ]]; then
    echo "✅ Loading configuration from deployment_config.json..."
    
    if [[ ! -f "deployment_config.json" ]]; then
        echo "❌ Error: deployment_config.json not found. Cannot resume deployment."
        exit 1
    fi

    # Extract and export all environment variables from JSON file
    while IFS= read -r line; do
        key=$(echo "$line" | cut -d '=' -f1)
        value=$(echo "$line" | cut -d '=' -f2-)
        
        # Ensure key and value are not empty
        if [[ -n "$key" && -n "$value" ]]; then
            export "$key"="$value"
        fi
    done < <(jq -r 'to_entries | .[] | "\(.key)=\(.value)"' deployment_config.json)

    echo "✅ Environment restored! Resuming deployment..."

    read -p "Have verifiers registered support for the chain? (yes/no): " VERIFIERS_REGISTERED

    if [[ "$VERIFIERS_REGISTERED" == "yes" ]]; then
        read -p "Have multisig proposals been approved? (yes/no): " MULTISIG_PROPOSALS_APPROVED
        if [[ "$MULTISIG_PROPOSALS_APPROVED" == "yes" ]]; then
            goto_after_multisig_proposals
        else
            goto_after_chain_registration
        fi
    else
        print_env_json_and_exit
    fi

    exit 0
fi

# Prompt user for required values
get_network_name
read -p "Enter Chain Name: " CHAIN_NAME
read -p "Enter Chain ID: " CHAIN_ID
read -p "Enter Token Symbol: " TOKEN_SYMBOL
read -p "Gas Limit: " GAS_LIMIT

validate_private_key
validate_rpc_url
validate_axelar_rpc_url
read -p "Enter Axelar Network Wallet MNEMONIC: " MNEMONIC
read -p "Enter version to retrieve (leave empty for latest): " USER_VERSION

# Export values as environment variables
export NAMESPACE
export CHAIN_NAME
export CHAIN_ID
export TOKEN_SYMBOL
export GAS_LIMIT
export TARGET_CHAIN_PRIVATE_KEY
export RPC_URL
export AXELAR_RPC_URL
export MNEMONIC

echo "✅ Environment Variables Set:"
echo "   NETWORK=$NAMESPACE"
echo "   CHAIN_NAME=$CHAIN_NAME"
echo "   CHAIN_ID=$CHAIN_ID"
echo "   TOKEN_SYMBOL=$TOKEN_SYMBOL"
echo "   GAS_LIMIT=$GAS_LIMIT"
echo "   TARGET_CHAIN_PRIVATE_KEY=$TARGET_CHAIN_PRIVATE_KEY"
echo "   MNEMONIC=$MNEMONIC"
echo "   RPC_URL=$RPC_URL"
echo "   AXELAR_RPC_URL=$AXELAR_RPC_URL"

# Create entry into name space json
generate_json_config
insert_into_network_config




# Check if the namespace is a custom devnet
if is_custom_devnet; then
    echo "🔧 Custom devnet detected. Proceeding with full deployment flow..."
    # Proceed with contract deployment as usual
    create_wallet
   
    # Ensure the directory for downloads exists
    mkdir -p "../wasm"

    # List of contract directories to check
    CONTRACT_DIRECTORIES=(
        "gateway"
        "multisig-prover"
        "voting-verifier"
    )

    # Loop through each contract directory and get the latest available version
    for dir in "${CONTRACT_DIRECTORIES[@]}"; do
        file_name="${dir//-/_}"  # Convert hyphens to underscores

        if [[ -z "$USER_VERSION" ]]; then
            get_latest_version "$dir"
        else
            FILE_URL="https://static.axelar.network/releases/cosmwasm/$dir/$USER_VERSION/${file_name}.wasm"
            CHECKSUM_URL="https://static.axelar.network/releases/cosmwasm/$dir/$USER_VERSION/checksums.txt"

            echo "⬇️ Downloading $FILE_URL..."
            
            # Ensure the directory exists before downloading
            mkdir -p "../wasm"

            wget -q "$FILE_URL" -O "../wasm/${file_name}.wasm"

            # Check if the file is empty after download
            if [[ ! -s "../wasm/${file_name}.wasm" ]]; then
                echo "⚠️ Warning: Downloaded file is empty! Removing it..."
                rm "../wasm/${file_name}.wasm"
            else
                echo "✅ Downloaded ${file_name}.wasm successfully!"
            fi

            wget -q "$CHECKSUM_URL" -O "../wasm/${file_name}_checksums.txt"

            # Check if the file is empty after download
            if [[ ! -s "../wasm/${file_name}_checksums.txt" ]]; then
                echo "⚠️ Warning: Downloaded file is empty! Removing it..."
                rm "../wasm/${file_name}_checksums.txt"
            else
                echo "✅ Downloaded ${file_name}_checksums.txt successfully!"
            fi
        fi
    done

    # Run the command to get the governance address
    export GOVERNANCE_ADDRESS=$(jq -r '.axelar.contracts.ServiceRegistry.governanceAccount' ../axelar-chains-config/info/"$NAMESPACE".json)
    export ADMIN_ADDRESS="$GOVERNANCE_ADDRESS"
    export CONTRACT_ADMIN="$GOVERNANCE_ADDRESS"
    export PROVER_ADMIN="$GOVERNANCE_ADDRESS"
    export DEPLOYER="$GOVERNANCE_ADDRESS"
    export SERVICE_NAME="validators"
    export VOTING_THRESHOLD='["6", "10"]'
    export SIGNING_THRESHOLD='["6", "10"]'
    export CONFIRMATION_HEIGHT="1"
    export MINIMUM_ROTATION_DELAY="0"
    export DEPLOYMENT_TYPE="create"
    export DEPOSIT_VALUE="100000000"
    echo "✅ Extracted GOVERNANCE_ADDRESS: $GOVERNANCE_ADDRESS"
    echo "✅ Extracted ADMIN_ADDRESS: $ADMIN_ADDRESS"



else
    echo "🚀 Predefined network detected ($NAMESPACE). Using existing governance and admin addresses."
    
    set_predefined_values

    # Display the reused values for confirmation
    echo "✅ Predefined values set for $NAMESPACE:"
    echo "   GOVERNANCE_ADDRESS=$GOVERNANCE_ADDRESS"
    echo "   ADMIN_ADDRESS=$ADMIN_ADDRESS"
    echo "   SERVICE_NAME=$SERVICE_NAME"
    echo "   VOTING_THRESHOLD=$VOTING_THRESHOLD"
    echo "   SIGNING_THRESHOLD=$SIGNING_THRESHOLD"
    echo "   CONFIRMATION_HEIGHT=$CONFIRMATION_HEIGHT"
    echo "   MINIMUM_ROTATION_DELAY=$MINIMUM_ROTATION_DELAY"
    echo "   DEPLOYMENT_TYPE=$DEPLOYMENT_TYPE"
    echo "   DEPLOYER=$DEPLOYER"


fi

# Run the deployment script and capture the output
echo "⚡ Running deploy-amplifier-gateway.js..."
setup_output=$(node ../evm/deploy-amplifier-gateway.js --env "$NAMESPACE" -n "$CHAIN_NAME" -m "$DEPLOYMENT_TYPE" --minimumRotationDelay "$MINIMUM_ROTATION_DELAY"  --predictOnly -p "$TARGET_CHAIN_PRIVATE_KEY" 2>&1)

# Print output for debugging
echo "$setup_output"

# Extract the predicted gateway proxy address
extract_proxy_gateway_address "$setup_output"

# Call the function to update JSON
update_voting_verifier_config
update_multisig_prover

if is_custom_devnet; then
    # Extract SALT for "VotingVerifier"
    extract_salt "voting_verifier"

    # Run the deployment command
    echo "⚡ Deploying VotingVerifier Contract..."
    node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "$MNEMONIC" \
        -a "$WASM_DIR" \
        -c "VotingVerifier" \
        -e "$NAMESPACE" \
        -n "$CHAIN_NAME" \
        --admin "$CONTRACT_ADMIN" \
        -y \
        --salt "$SALT"


    # Extract SALT for "Gateway"
    extract_salt "gateway"

    # Run the deployment command for Gateway contract
    echo "⚡ Deploying Gateway Contract..."
    node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "$MNEMONIC" \
        -a "$WASM_DIR" \
        -c "Gateway" \
        -e "$NAMESPACE" \
        -n "$CHAIN_NAME" \
        --admin "$CONTRACT_ADMIN" \
        -y \
        --salt "$SALT"

    # Extract SALT for "MultisigProver"
    extract_salt "multisig_prover"

    # Run the deployment command for MultisigProver contract
    echo "⚡ Deploying MultisigProver Contract..."

    node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "$MNEMONIC" \
        -a "$WASM_DIR" \
        -c "MultisigProver" \
        -e "$NAMESPACE" \
        -n "$CHAIN_NAME" \
        --admin "$CONTRACT_ADMIN" \
        -y \
        --salt "$SALT"

    
    # Function to retrieve the wallet address
    get_wallet_address() {
        WALLET_ADDRESS=$(axelard keys show amplifier --keyring-backend test | awk '/address:/ {print $2}')
        
        if [[ -z "$WALLET_ADDRESS" ]]; then
            echo "❌ Could not retrieve wallet address!"
            exit 1
        fi

        echo "✅ Retrieved wallet address: $WALLET_ADDRESS"
    }

    # Function to determine the token denomination
    get_token_denomination() {
        echo "⚡ Querying wallet balance to determine token denomination..."

        balance_output=$(axelard q bank balances "$WALLET_ADDRESS" --node "$AXELAR_RPC_URL" 2>&1)

        # Print raw output for debugging
        echo "🔍 Wallet Balance Output:"
        echo "$balance_output"

        # Extract the first token denomination found
        TOKEN_DENOM=$(echo "$balance_output" | awk '/denom:/ {print $2}' | head -n 1)

        if [[ -z "$TOKEN_DENOM" ]]; then
            echo "❌ Could not determine token denomination! Check if wallet has funds."
            exit 1
        fi

        echo "✅ Retrieved token denomination: $TOKEN_DENOM"
    }

    # Run functions
    get_wallet_address
    get_token_denomination

    # Store the extracted denomination in an ENV variable
    export TOKEN_DENOM

    echo "✅ Set TOKEN_DENOM=$TOKEN_DENOM"



    
else
    node ./../cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
    node ./../cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
    node ./../cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN

fi


# Run the functions to extract values
extract_router_address
extract_gateway_address
build_json_cmd_register


if is_custom_devnet; then
    # Run the command to register the chain
    echo "⚡ Registering the chain..."
    axelard tx wasm execute "$ROUTER_ADDRESS" "$JSON_CMD_REGISTER" \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "$AXELAR_RPC_URL" \
        --gas-prices 0.00005"$TOKEN_DENOM" \
        --keyring-backend test \
        --chain-id "$NAMESPACE"
    # Print the output for debugging
    echo "$register_output"
else
    if $NAMESPACE = "devnet-amplifier"; then
        node ../cosmwasm/submit-proposal.js execute \
            -c Router \
            -t "Register Gateway for $CHAIN_NAME" \
            -d "Register Gateway address for $CHAIN_NAME at Router contract" \
            --runAs $RUN_AS_ACCOUNT \
            --deposit $DEPOSIT_VALUE \
            --msg "$JSON_CMD_REGISTER"
    else
        node ../cosmwasm/submit-proposal.js execute \
            -c Router \
            -t "Register Gateway for $CHAIN_NAME" \
            -d "Register Gateway address for $CHAIN_NAME at Router contract" \
            --deposit $DEPOSIT_VALUE \
            --msg "$JSON_CMD_REGISTER"
    fi
fi

# Generate extra envs for next steps needed as part of verifeir set
REWARDS_ADDRESS=$(cat ../axelar-chains-config/info/"$NAMESPACE".json | jq -rM '.axelar.contracts.Rewards.address');
export REWARDS_ADDRESS;
MULTISIG_ADDRESS=$(cat ../axelar-chains-config/info/"$NAMESPACE".json | jq -rM '.axelar.contracts.Multisig.address');
export MULTISIG_ADDRESS;

JSON_PATH=".axelar.contracts.VotingVerifier.${CHAIN_NAME}.address";
VOTING_VERIFIER_ADDRESS=$(cat ../axelar-chains-config/info/"$NAMESPACE".json | jq -rM "$JSON_PATH"); echo $VOTING_VERIFIER_ADDRESS;
export VOTING_VERIFIER_ADDRESS;


echo "🎉 Chain registration complete! Need to Update the Verifiers!"

print_env_json_and_exit

