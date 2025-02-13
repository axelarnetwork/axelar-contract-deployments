#!/bin/bash

# Environment Variables

NAMESPACE="testnet"
NAMESPACE2="testnet-amplifiers"
WORKER="ampd-axelar-amplifier-worker-0"
BASTION="axelar-core-bastion-bastion-6f5ddc97c5-jl45c"
BASTION_CONTAINER="axelar-core-bastion"
SERVICE_NAME="amplifier"
MULTISIG_PROVER_ADDRESS="axelar1v8jrupu2rqpskwgtr69max0ajul92q8z5mdxd505m2hu3xc5jzcqm8zyc6"
VOTING_VERIFIER_ADDRESS="axelar1sykyha8kzf35kc5hplqk76kdufntjn6w45ntwlevwxp74dqr3rvsq7fazh"
SERVICE_REGISTRY="axelar1rpj2jjrv3vpugx9ake9kgk3s2kgwt0y60wtkmcgfml5m3et0mrls6nct9m"

# On mainnet there will be a rotation delay, so we can only run one iteration at a time
ITERATIONS=1

print_section() {
    echo ""
    echo ""
    echo "============================== $1 =============================="
}

print_success() {
    echo ""
    echo "[✔] $1"
}

print_info() {
    echo ""
    echo "[INFO]: $1"
}

# Functions
set_namespace() {
    print_section "Setting Namespace"
    kubectl config set-context --current --namespace="$NAMESPACE"
    print_success "Namespace set to $NAMESPACE"
}

query_current_verifier_set() {
    kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- bash -c "axelard q wasm contract-state smart $1 '\"current_verifier_set\"'"
}

query_next_verifier_set() {
    kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- bash -c "axelard q wasm contract-state smart $1 '\"next_verifier_set\"' -o json"
}

get_worker_address() {
    kubectl exec -it "$WORKER" -n "$NAMESPACE2" -c ampd-verifier-address -- bash -c "cat verifier-address"
}

query_active_verifiers() {
    kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- bash -c \
        "axelard q wasm contract-state smart $SERVICE_REGISTRY '{\"active_verifiers\":{\"service_name\": \"$SERVICE_NAME\",\"chain_name\":\"sui\"}}'"
}

register_or_deregister() {
    if echo "$ACTIVE_VERIFIER_SET" | grep -q "$WORKER_ADDRESS"; then
        print_info "Deregistering support for the chain..."
        kubectl exec -it "$WORKER" -n "$NAMESPACE2" -c ampd -- bash -c "ampd deregister-chain-support $SERVICE_NAME sui"
    else
        print_success "Registering support for the chain..."
        kubectl exec -it "$WORKER" -n "$NAMESPACE2" -c ampd -- bash -c "ampd register-chain-support $SERVICE_NAME sui"
    fi
}

update_verifier_set() {
    kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- sh -c \
        "echo \$KEYRING_PASSWORD | axelard tx wasm execute $MULTISIG_PROVER_ADDRESS '\"update_verifier_set\"' --from multisig-prover-admin --output json --gas auto --gas-adjustment 1.4 -y"
}

query_proof_status() {
    while true; do
        local status
        status=$(kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- bash -c \
            "axelard q wasm contract-state smart $MULTISIG_PROVER_ADDRESS '{\"proof\":{\"multisig_session_id\":\"$1\"}}'")

        print_info "Proof Status: $status"
        if echo "$status" | grep -q "completed"; then
            print_success "Proof completed."
            break
        fi
        sleep 5
    done
}

query_verifier_set_status() {
    while true; do
        local status
        status=$(kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- bash -c \
            "axelard q wasm contract-state smart $VOTING_VERIFIER_ADDRESS '{\"verifier_set_status\": $1}'")

        print_info "Verifier Set Status: $status"
        if echo "$status" | grep -q "succeeded_on_source_chain"; then
            print_success "Verifier set update succeeded."
            break
        fi
        sleep 5
    done
}

confirm_verifier_set() {
    kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- bash -c \
        "echo \$KEYRING_PASSWORD | axelard tx wasm execute $MULTISIG_PROVER_ADDRESS '\"confirm_verifier_set\"' --from validator --gas auto --gas-adjustment 1.4 -y"
}

# Main Loop
set_namespace

for ((i = 1; i <= ITERATIONS; i++)); do
    print_info "Iteration $i"

    # Step 1: Query Current Verifier Set
    print_section "1. Querying Verifier Set"
    VERIFIER_SET=$(query_current_verifier_set "$MULTISIG_PROVER_ADDRESS")
    echo "Current Verifier Set: $VERIFIER_SET"

    # Step 2: Register/Deregister
    print_section "2. Register/Deregister Verifier"
    WORKER_ADDRESS=$(get_worker_address)
    print_info "Worker address: $WORKER_ADDRESS"

    ACTIVE_VERIFIER_SET=$(query_active_verifiers)
    print_info "Active verifier set: $ACTIVE_VERIFIER_SET"
    register_or_deregister

    # Step 3: Update Verifier Set
    print_section "3. Updating Verifier Set"
    UPDATE_TX=$(update_verifier_set)
    MULTISIG_SESSION_ID=$(echo "$UPDATE_TX" | tail -n +2 | jq -r '.logs[].events[] | select(.type=="wasm-proof_under_construction") | .attributes[] | select(.key=="multisig_session_id") | .value' | tr -d '"')
    # MULTISIG_SESSION_ID=344
    print_success "Multisig Session ID: $MULTISIG_SESSION_ID"

    # Step 4: Query Proof Status
    print_section "4. Querying Proof Status"
    query_proof_status "$MULTISIG_SESSION_ID"

    # Step 5: Retrieve Event Sequence and Update Verifier Set
    print_section "5. Retrieving Event Sequence"
    print_info "Make sure you update the .env file to the correct environment before running the command"
    echo "Run the following command:"
    echo "node sui/gateway.js submitProof ${MULTISIG_SESSION_ID}"
    read -r TX_HASH
    MESSAGE_ID="${TX_HASH}-0"
    print_info "Message ID: $MESSAGE_ID"

    NEXT_VERIFIER_SET=$(query_next_verifier_set "$MULTISIG_PROVER_ADDRESS" | jq '.data.verifier_set' | tr -d '\n' | tr -s ' ')
    print_success "Next Verifier Set: $NEXT_VERIFIER_SET"

    # Step 6: verify verifier set
    print_section "6. Verifying Verifier Set"
    JSON_PAYLOAD="{\"verify_verifier_set\":{\"message_id\":\"$MESSAGE_ID\",\"new_verifier_set\":$NEXT_VERIFIER_SET}}"
    print_info "JSON Payload: $JSON_PAYLOAD"
    VERIFY_TX=$(kubectl exec -it "$BASTION" -c "$BASTION_CONTAINER" -- bash -c \
        "echo \$KEYRING_PASSWORD | axelard tx wasm execute $VOTING_VERIFIER_ADDRESS '$JSON_PAYLOAD' --from validator --gas auto --gas-adjustment 1.4 -y --output json")

    # Step 7: check source chain
    print_section "7. Check sourse chain"
    print_info "Verify Transaction: $VERIFY_TX"
    query_verifier_set_status "$NEXT_VERIFIER_SET"

    # Step 8: consirm verifier set on multisig prover
    print_section "8. Confirming verifier set on multisig prover contract"
    confirm_verifier_set
    NEW_VERIFIER_SET=$(query_current_verifier_set "$MULTISIG_PROVER_ADDRESS")
    print_info "$NEW_VERIFIER_SET"

    print_success "Iteration $i completed."
done

print_success "Process completed for $ITERATIONS iterations."
