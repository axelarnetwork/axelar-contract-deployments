#!/bin/bash
set -euo pipefail

# =============================================================================
# CosmWasm Solana GMP Amplifier Deployment Script
#
# Deploys CosmWasm Amplifier contracts for Solana GMP connection.
# Automates the steps in releases/cosmwasm/2025-09-Solana-GMP-v1.0.0.md.
#
# Run this script AFTER solana/deploy.sh deploys Solana programs.
# It deploys the CosmWasm side (VotingVerifier, Gateway, MultisigProver, etc.)
# then returns control to the user for Solana gateway initialization.
#
# Secrets & addresses:
#   - Mnemonic: fetched from 1Password, or $MNEMONIC env var
#   - CONTRACT_ADMIN: fetched from 1Password "[Env] Emergency Operator EOA: Axelar"
#   - PROVER_ADMIN: fetched from 1Password "[Env] Key Rotation EOA: Axelar"
#   - Governance/admin addresses: read from config JSON
#
# Prerequisites:
#   - ENV and CHAIN must be set (env vars or solana/.env)
#   - Solana gateway must be deployed (AxelarGateway.address in config)
#   - 1Password CLI (op) authenticated, or MNEMONIC env var set
#
# Usage:
#   ENV=stagenet CHAIN=solana ./solana/deploy-axelar.sh
#   ./solana/deploy-axelar.sh --reset                    # start fresh
#   ./solana/deploy-axelar.sh --version-vv 2.1.0         # override version
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOYMENTS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# --- Logging (same as solana/deploy.sh) ---
log_step()  { echo -e "\n\033[1;34m==> $1\033[0m"; }
log_info()  { echo "    $1"; }
log_warn()  { echo -e "    \033[1;33mWARNING: $1\033[0m"; }
log_error() { echo -e "\033[1;31mERROR: $1\033[0m"; }

confirm() {
    local prompt="${1:-Proceed?}"
    read -r -p "    ${prompt} [y/N] " response
    [[ "$response" =~ ^[Yy]$ ]]
}

# --- Source solana/.env if it exists (for ENV, CHAIN) ---
if [[ -f "${SCRIPT_DIR}/.env" ]]; then
    # shellcheck source=/dev/null
    source "${SCRIPT_DIR}/.env"
fi

# --- Validate ENV ---
case "${ENV:-}" in
    devnet-amplifier|stagenet|testnet|mainnet) ;;
    *)
        log_error "Invalid ENV='${ENV:-}'. Must be one of: devnet-amplifier, stagenet, testnet, mainnet"
        exit 1
        ;;
esac

if [[ -z "${CHAIN:-}" ]]; then
    log_error "CHAIN not set. Export CHAIN or set it in solana/.env"
    exit 1
fi

# =============================================================================
# Environment-dependent configuration
# =============================================================================

CHAINS_INFO_FILE="${DEPLOYMENTS_DIR}/axelar-chains-config/info/${ENV}.json"

get_op_vault() {
    case "$ENV" in
        devnet-amplifier) echo "Devnet - Axelar Externally Owned Accounts" ;;
        stagenet)         echo "Stagenet - Axelar Externally Owned Accounts" ;;
        testnet)          echo "Testnet - Axelar Externally Owned Accounts" ;;
        mainnet)          log_error "1Password vault for mainnet not configured yet"; exit 1 ;;
    esac
}

get_env_display() {
    case "$ENV" in
        devnet-amplifier) echo "Devnet" ;;
        stagenet)         echo "Stagenet" ;;
        testnet)          echo "Testnet" ;;
        mainnet)          echo "Mainnet" ;;
    esac
}

get_service_name() {
    case "$ENV" in
        devnet-amplifier) echo "validators" ;;
        *)                echo "amplifier" ;;
    esac
}

get_voting_threshold() {
    case "$ENV" in
        devnet-amplifier) echo '["6","10"]' ;;
        stagenet|testnet) echo '["51","100"]' ;;
        mainnet)          echo '["2","3"]' ;;
    esac
}

get_signing_threshold() {
    get_voting_threshold
}

get_salt() {
    echo "v1.0.0"
}

get_epoch_duration() {
    case "$ENV" in
        devnet-amplifier) echo "100" ;;
        stagenet)         echo "600" ;;
        testnet|mainnet)  echo "14845" ;;
    esac
}

get_participation_threshold() {
    case "$ENV" in
        mainnet) echo '["8","10"]' ;;
        *)       echo '["7","10"]' ;;
    esac
}

get_rewards_per_epoch() {
    echo "100"
}

get_reward_amount() {
    case "$ENV" in
        devnet-amplifier) echo "1000000uamplifier" ;;
        *)                echo "1000000uaxl" ;;
    esac
}

OP_VAULT=$(get_op_vault)
ENV_DISPLAY=$(get_env_display)

# Governance flag: empty on devnet (direct execution), --governance elsewhere
if [[ "$ENV" == "devnet-amplifier" ]]; then
    GOVERNANCE_FLAG=""
    REQUIRES_GOVERNANCE=false
else
    GOVERNANCE_FLAG="--governance"
    REQUIRES_GOVERNANCE=true
fi

# =============================================================================
# Flag parsing
# =============================================================================

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Deploys CosmWasm Amplifier contracts for Solana GMP connection."
    echo "Reads ENV and CHAIN from environment or solana/.env."
    echo ""
    echo "Options:"
    echo "  --reset                  Clear state file and start from scratch"
    echo "  --version-vv <ver>       VotingVerifier version   (default: 2.0.0)"
    echo "  --version-gw <ver>       Gateway version          (default: 1.1.1)"
    echo "  --version-mp <ver>       MultisigProver version   (default: 2.0.0)"
    echo "  --version-its <ver>      ItsSolanaTranslator ver  (default: 1.0.0)"
    echo "  -h, --help               Show this help"
    echo ""
    echo "Environment variables:"
    echo "  ENV              Required: devnet-amplifier, stagenet, testnet, mainnet"
    echo "  CHAIN            Required: solana chain name (e.g. solana)"
    echo "  MNEMONIC         Fallback if 1Password unavailable"
    echo "  NODE             Axelar RPC URL (reads from config if unset)"
    echo "  REWARDS_WALLET   axelard keyring name for funding reward pools"
}

VOTING_VERIFIER_VERSION="2.0.0"
GATEWAY_VERSION="1.1.1"
MULTISIG_PROVER_VERSION="2.0.0"
ITS_TRANSLATOR_VERSION="1.0.0"
RESET=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --reset)
            RESET=true
            shift
            ;;
        --version-vv)
            VOTING_VERIFIER_VERSION="$2"
            shift 2
            ;;
        --version-gw)
            GATEWAY_VERSION="$2"
            shift 2
            ;;
        --version-mp)
            MULTISIG_PROVER_VERSION="$2"
            shift 2
            ;;
        --version-its)
            ITS_TRANSLATOR_VERSION="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown flag: $1"
            usage
            exit 1
            ;;
    esac
done

# =============================================================================
# State management (resume support)
# =============================================================================

STATE_FILE="${DEPLOYMENTS_DIR}/.deploy-axelar-state-${ENV}-${CHAIN}"

if [[ "$RESET" == "true" ]]; then
    rm -f "$STATE_FILE"
    log_info "State file cleared."
fi

is_step_done() {
    local step="$1"
    [[ -f "$STATE_FILE" ]] && grep -qx "$step" "$STATE_FILE"
}

mark_step_done() {
    local step="$1"
    echo "$step" >> "$STATE_FILE"
}

# =============================================================================
# Secret & address resolution
# =============================================================================

fetch_op_field() {
    local title="$1"
    local field="$2"
    op item get "$title" --vault "$OP_VAULT" --fields "$field" --reveal 2>/dev/null
}

resolve_mnemonic() {
    # Try 1Password first
    if command -v op &>/dev/null; then
        local mnemonic
        if mnemonic=$(fetch_op_field "[${ENV_DISPLAY}] Deployer EOA: Axelar" "Mnemonic"); then
            MNEMONIC="$mnemonic"
            log_info "Mnemonic loaded from 1Password"
            return
        fi
        log_warn "Could not fetch mnemonic from 1Password, falling back to \$MNEMONIC"
    fi

    if [[ -z "${MNEMONIC:-}" ]]; then
        log_error "MNEMONIC not set. Set it via environment or configure 1Password."
        log_info "1Password item: [${ENV_DISPLAY}] Deployer EOA: Axelar (field: Mnemonic)"
        exit 1
    fi
    log_info "Using mnemonic from environment"
}

resolve_contract_admin() {
    if command -v op &>/dev/null; then
        local addr
        if addr=$(fetch_op_field "[${ENV_DISPLAY}] Emergency Operator EOA: Axelar" "Address"); then
            CONTRACT_ADMIN="$addr"
            log_info "Contract admin (Emergency Operator): $CONTRACT_ADMIN"
            return
        fi
        log_warn "Could not fetch Emergency Operator from 1Password"
    fi

    if [[ -z "${CONTRACT_ADMIN:-}" ]]; then
        log_error "CONTRACT_ADMIN not set. Set it via environment or configure 1Password."
        log_info "1Password item: [${ENV_DISPLAY}] Emergency Operator EOA: Axelar (field: Address)"
        exit 1
    fi
}

resolve_prover_admin() {
    if command -v op &>/dev/null; then
        local addr
        if addr=$(fetch_op_field "[${ENV_DISPLAY}] Key Rotation EOA: Axelar" "Address"); then
            PROVER_ADMIN="$addr"
            log_info "Prover admin (Key Rotation): $PROVER_ADMIN"
            return
        fi
        log_warn "Could not fetch Key Rotation from 1Password"
    fi

    if [[ -z "${PROVER_ADMIN:-}" ]]; then
        log_error "PROVER_ADMIN not set. Set it via environment or configure 1Password."
        log_info "1Password item: [${ENV_DISPLAY}] Key Rotation EOA: Axelar (field: Address)"
        exit 1
    fi
}

# =============================================================================
# Config helpers
# =============================================================================

jq_config() {
    jq -r "$1" "$CHAINS_INFO_FILE"
}

load_config_values() {
    log_step "Loading configuration from ${ENV}.json"

    GOVERNANCE_ADDRESS=$(jq_config '.axelar.governanceAddress')
    ADMIN_ADDRESS=$(jq_config '.axelar.adminAddress // empty')
    COORDINATOR_ADDRESS=$(jq_config '.axelar.contracts.Coordinator.address // empty')

    NODE="${NODE:-$(jq_config '.axelar.rpc // empty')}"
    if [[ -z "$NODE" ]]; then
        log_error "NODE not set and axelar.rpc not found in config"
        exit 1
    fi

    log_info "Governance:  $GOVERNANCE_ADDRESS"
    log_info "Admin:       ${ADMIN_ADDRESS:-<not set>}"
    log_info "Coordinator: ${COORDINATOR_ADDRESS:-<not set>}"
    log_info "Node:        $NODE"
}

build_init_addresses() {
    # Construct INIT_ADDRESSES from config's govProposalInstantiateAddresses + Coordinator
    local config_addrs
    config_addrs=$(jq_config '[.axelar.govProposalInstantiateAddresses // [] | .[]] | join(",")')

    if [[ -n "$COORDINATOR_ADDRESS" ]]; then
        if [[ -n "$config_addrs" ]]; then
            INIT_ADDRESSES="${config_addrs},${COORDINATOR_ADDRESS}"
        else
            INIT_ADDRESSES="$COORDINATOR_ADDRESS"
        fi
    else
        INIT_ADDRESSES="$config_addrs"
    fi

    if [[ -z "$INIT_ADDRESSES" ]]; then
        log_error "Could not construct INIT_ADDRESSES (no govProposalInstantiateAddresses or Coordinator in config)"
        exit 1
    fi

    log_info "Init addresses: $INIT_ADDRESSES"
}

load_deployed_addresses() {
    VOTING_VERIFIER=$(jq_config ".axelar.contracts.VotingVerifier[\"${CHAIN}\"].address // empty")
    GATEWAY_CW=$(jq_config ".axelar.contracts.Gateway[\"${CHAIN}\"].address // empty")
    MULTISIG_PROVER=$(jq_config ".axelar.contracts.MultisigProver[\"${CHAIN}\"].address // empty")
    MULTISIG=$(jq_config '.axelar.contracts.Multisig.address // empty')
    REWARDS=$(jq_config '.axelar.contracts.Rewards.address // empty')
    ROUTER=$(jq_config '.axelar.contracts.Router.address // empty')
    SERVICE_REGISTRY=$(jq_config '.axelar.contracts.ServiceRegistry.address // empty')
}

# =============================================================================
# Wrappers
# =============================================================================

run_ts_node() {
    log_info "Running: ts-node $*"
    (cd "$DEPLOYMENTS_DIR" && ts-node "$@")
}

run_axelard() {
    log_info "Running: axelard $*"
    axelard "$@"
}

wait_for_proposal() {
    local description="$1"

    if [[ "$REQUIRES_GOVERNANCE" != "true" ]]; then
        log_info "No governance required on $ENV. Proceeding."
        return
    fi

    echo ""
    log_step "Governance Proposal: $description"
    echo "    The governance proposal has been submitted."
    echo "    Please wait for it to pass before continuing."
    echo ""
    echo "    If the proposal fails, you can re-run this script."
    echo "    It will resume from the next step."
    echo ""

    if ! confirm "Has the governance proposal passed?"; then
        log_warn "Script paused. Re-run to resume from this point."
        exit 0
    fi
}

# =============================================================================
# Prerequisites
# =============================================================================

check_prerequisites() {
    log_step "Checking prerequisites"

    local missing=()
    command -v ts-node >/dev/null 2>&1 || missing+=("ts-node")
    command -v jq >/dev/null 2>&1 || missing+=("jq")
    command -v node >/dev/null 2>&1 || missing+=("node")
    command -v axelard >/dev/null 2>&1 || missing+=("axelard")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi

    if ! command -v op >/dev/null 2>&1; then
        log_warn "1Password CLI (op) not found. Will rely on environment variables."
    fi

    log_info "All tools available"

    if [[ ! -f "$CHAINS_INFO_FILE" ]]; then
        log_error "Chains info file not found: $CHAINS_INFO_FILE"
        exit 1
    fi

    # Validate chain entry exists
    local chain_entry
    chain_entry=$(jq_config ".chains[\"${CHAIN}\"] // empty")
    if [[ -z "$chain_entry" ]]; then
        log_error "Chain '${CHAIN}' not found in ${ENV}.json"
        log_info "Run solana/deploy.sh first to create the chain entry."
        exit 1
    fi

    # Validate Solana gateway is deployed
    local gw_addr
    gw_addr=$(jq_config ".chains[\"${CHAIN}\"].contracts.AxelarGateway.address // empty")
    if [[ -z "$gw_addr" ]]; then
        log_error "AxelarGateway address not found for chain '${CHAIN}'"
        log_info "Ensure Solana gateway programs are deployed before running this script."
        exit 1
    fi
    log_info "Solana AxelarGateway: $gw_addr"

    # Warn if ChainCodecSolana not deployed
    local codec_addr
    codec_addr=$(jq_config '.axelar.contracts.ChainCodecSolana.address // empty')
    if [[ -z "$codec_addr" ]]; then
        log_warn "ChainCodecSolana contract not found in config."
        log_warn "This is required for instantiate-chain-contracts (step 9)."
        if ! confirm "Continue anyway?"; then
            exit 1
        fi
    fi

    log_info "All prerequisites verified"
}

# =============================================================================
# Phase 1: Store Contracts
# =============================================================================

step_store_voting_verifier() {
    local STEP_NAME="store_voting_verifier"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 1: Store VotingVerifier (v${VOTING_VERIFIER_VERSION})"

    if ! confirm "Submit store-code for VotingVerifier?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts store-code \
        -c VotingVerifier \
        -t "Upload VotingVerifier contract for Solana" \
        -d "Upload VotingVerifier contract for Solana integration" \
        -v "$VOTING_VERIFIER_VERSION" \
        --chainName "$CHAIN" \
        -m "$MNEMONIC" \
        --instantiateAddresses "$INIT_ADDRESSES" \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "VotingVerifier store-code"
}

step_store_gateway() {
    local STEP_NAME="store_gateway"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 2: Store Gateway (v${GATEWAY_VERSION})"

    if ! confirm "Submit store-code for Gateway?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts store-code \
        -c Gateway \
        -t "Upload Gateway contract for Solana" \
        -d "Upload Gateway contract for Solana integration" \
        -v "$GATEWAY_VERSION" \
        --chainName "$CHAIN" \
        -m "$MNEMONIC" \
        --instantiateAddresses "$INIT_ADDRESSES" \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "Gateway store-code"
}

step_store_multisig_prover() {
    local STEP_NAME="store_multisig_prover"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 3: Store MultisigProver (v${MULTISIG_PROVER_VERSION})"

    if ! confirm "Submit store-code for MultisigProver?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts store-code \
        -c MultisigProver \
        -t "Upload MultisigProver contract for Solana" \
        -d "Upload MultisigProver contract for Solana integration" \
        -v "$MULTISIG_PROVER_VERSION" \
        --chainName "$CHAIN" \
        -m "$MNEMONIC" \
        --instantiateAddresses "$INIT_ADDRESSES" \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "MultisigProver store-code"
}

step_store_its_solana_translator() {
    local STEP_NAME="store_its_translator"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 4: Store ItsSolanaTranslator (v${ITS_TRANSLATOR_VERSION})"

    local existing_addr
    existing_addr=$(jq_config '.axelar.contracts.ItsSolanaTranslator.address // empty')
    if [[ -n "$existing_addr" ]]; then
        log_info "ItsSolanaTranslator already deployed at: $existing_addr"
        if ! confirm "Store new code anyway?"; then
            mark_step_done "$STEP_NAME"
            return
        fi
    fi

    if ! confirm "Submit store-code for ItsSolanaTranslator?"; then
        log_info "Skipping."
        return
    fi

    # No --chainName: ItsSolanaTranslator is CONTRACT_SCOPE_GLOBAL
    # shellcheck disable=SC2086
    CHAIN= run_ts_node cosmwasm/contract.ts store-code \
        -c ItsSolanaTranslator \
        -t "Upload ItsSolanaTranslator contract v${ITS_TRANSLATOR_VERSION}" \
        -d "Upload ItsSolanaTranslator contract v${ITS_TRANSLATOR_VERSION}" \
        -v "$ITS_TRANSLATOR_VERSION" \
        -m "$MNEMONIC" \
        --instantiateAddresses "$INIT_ADDRESSES" \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "ItsSolanaTranslator store-code"
}

step_instantiate_its_translator() {
    local STEP_NAME="instantiate_its_translator"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 5: Instantiate ItsSolanaTranslator"

    local existing_addr
    existing_addr=$(jq_config '.axelar.contracts.ItsSolanaTranslator.address // empty')
    if [[ -n "$existing_addr" ]]; then
        log_info "ItsSolanaTranslator already instantiated at: $existing_addr"
        mark_step_done "$STEP_NAME"
        return
    fi

    if ! confirm "Submit instantiate for ItsSolanaTranslator?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts instantiate \
        -c ItsSolanaTranslator \
        -m "$MNEMONIC" \
        --fetchCodeId \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "ItsSolanaTranslator instantiate"
}

step_verify_its_translator_address() {
    local STEP_NAME="verify_its_translator_address"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 6: Verify ItsSolanaTranslator address"

    local its_addr
    its_addr=$(jq_config '.axelar.contracts.ItsSolanaTranslator.address // empty')

    if [[ -n "$its_addr" ]]; then
        log_info "ItsSolanaTranslator address: $its_addr"
        mark_step_done "$STEP_NAME"
        return
    fi

    # Query on-chain to find the address
    local code_id
    code_id=$(jq_config '.axelar.contracts.ItsSolanaTranslator.codeId // empty')
    if [[ -n "$code_id" ]]; then
        log_info "Querying contracts for code ID: $code_id"
        run_axelard q wasm list-contract-by-code "$code_id" --node "$NODE" --output json | jq . || true
    fi

    echo ""
    log_warn "ItsSolanaTranslator address not found in config."
    log_info "If the proposal has passed, update axelar.contracts.ItsSolanaTranslator.address in ${ENV}.json"

    if ! confirm "Has the address been updated in the config?"; then
        log_warn "Script paused. Update the config and re-run."
        exit 0
    fi

    mark_step_done "$STEP_NAME"
}

# =============================================================================
# Phase 2: Configure & Deploy
# =============================================================================

step_add_voting_verifier_config() {
    local STEP_NAME="add_voting_verifier_config"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 7: Add VotingVerifier config for $CHAIN"

    local existing
    existing=$(jq_config ".axelar.contracts.VotingVerifier[\"${CHAIN}\"] // empty")
    if [[ -n "$existing" ]]; then
        log_info "VotingVerifier[$CHAIN] already exists in config. Skipping."
        mark_step_done "$STEP_NAME"
        return
    fi

    local source_gw_addr
    source_gw_addr=$(jq_config ".chains[\"${CHAIN}\"].contracts.AxelarGateway.address // empty")
    if [[ -z "$source_gw_addr" ]]; then
        log_error "AxelarGateway address not found in chains.${CHAIN}.contracts.AxelarGateway"
        exit 1
    fi

    local service_name voting_threshold
    service_name=$(get_service_name)
    voting_threshold=$(get_voting_threshold)

    log_info "governanceAddress:  $GOVERNANCE_ADDRESS"
    log_info "serviceName:        $service_name"
    log_info "sourceGatewayAddr:  $source_gw_addr"
    log_info "votingThreshold:    $voting_threshold"

    if ! confirm "Add VotingVerifier config for $CHAIN?"; then
        log_info "Skipping."
        return
    fi

    local tmp_file="${CHAINS_INFO_FILE}.tmp"
    jq --arg chain "$CHAIN" \
       --arg gov "$GOVERNANCE_ADDRESS" \
       --arg svc "$service_name" \
       --arg gw "$source_gw_addr" \
       --argjson thresh "$voting_threshold" \
    '.axelar.contracts.VotingVerifier[$chain] = {
        "governanceAddress": $gov,
        "serviceName": $svc,
        "sourceGatewayAddress": $gw,
        "votingThreshold": $thresh,
        "blockExpiry": 10,
        "confirmationHeight": 1000000,
        "msgIdFormat": "base58_solana_tx_signature_and_event_index",
        "addressFormat": "solana"
    }' "$CHAINS_INFO_FILE" > "$tmp_file" && mv "$tmp_file" "$CHAINS_INFO_FILE"

    log_info "VotingVerifier[$CHAIN] config added to ${ENV}.json"
    mark_step_done "$STEP_NAME"
}

step_add_multisig_prover_config() {
    local STEP_NAME="add_multisig_prover_config"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 8: Add MultisigProver config for $CHAIN"

    local existing
    existing=$(jq_config ".axelar.contracts.MultisigProver[\"${CHAIN}\"] // empty")
    if [[ -n "$existing" ]]; then
        log_info "MultisigProver[$CHAIN] already exists in config. Skipping."
        mark_step_done "$STEP_NAME"
        return
    fi

    local service_name signing_threshold
    service_name=$(get_service_name)
    signing_threshold=$(get_signing_threshold)

    log_info "governanceAddress:  $GOVERNANCE_ADDRESS"
    log_info "adminAddress:       ${ADMIN_ADDRESS:-$GOVERNANCE_ADDRESS}"
    log_info "signingThreshold:   $signing_threshold"
    log_info "serviceName:        $service_name"

    if ! confirm "Add MultisigProver config for $CHAIN?"; then
        log_info "Skipping."
        return
    fi

    local admin_addr="${ADMIN_ADDRESS:-$GOVERNANCE_ADDRESS}"
    local tmp_file="${CHAINS_INFO_FILE}.tmp"
    jq --arg chain "$CHAIN" \
       --arg gov "$GOVERNANCE_ADDRESS" \
       --arg admin "$admin_addr" \
       --argjson thresh "$signing_threshold" \
       --arg svc "$service_name" \
    '.axelar.contracts.MultisigProver[$chain] = {
        "governanceAddress": $gov,
        "adminAddress": $admin,
        "signingThreshold": $thresh,
        "serviceName": $svc,
        "verifierSetDiffThreshold": 0,
        "encoder": "solana",
        "keyType": "ecdsa"
    }' "$CHAINS_INFO_FILE" > "$tmp_file" && mv "$tmp_file" "$CHAINS_INFO_FILE"

    log_info "MultisigProver[$CHAIN] config added to ${ENV}.json"
    mark_step_done "$STEP_NAME"
}

step_instantiate_chain_contracts() {
    local STEP_NAME="instantiate_chain_contracts"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 9: Instantiate chain contracts (Gateway, VotingVerifier, MultisigProver)"

    local salt
    salt=$(get_salt)

    log_info "Chain:          $CHAIN"
    log_info "Salt:           $salt"
    log_info "Contract Admin: $CONTRACT_ADMIN"

    if ! confirm "Submit instantiate-chain-contracts?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts instantiate-chain-contracts \
        -n "$CHAIN" \
        -s "$salt" \
        --fetchCodeId \
        --admin "$CONTRACT_ADMIN" \
        -m "$MNEMONIC" \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "instantiate-chain-contracts"
}

step_save_deployed_contracts() {
    local STEP_NAME="save_deployed_contracts"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 10: Save deployed contract addresses"

    run_ts_node cosmwasm/query.ts save-deployed-contracts "$CHAIN"

    # Reload addresses from updated config
    load_deployed_addresses

    if [[ -n "$VOTING_VERIFIER" ]]; then
        log_info "VotingVerifier:  $VOTING_VERIFIER"
    fi
    if [[ -n "$GATEWAY_CW" ]]; then
        log_info "Gateway:         $GATEWAY_CW"
    fi
    if [[ -n "$MULTISIG_PROVER" ]]; then
        log_info "MultisigProver:  $MULTISIG_PROVER"
    fi

    mark_step_done "$STEP_NAME"
}

step_register_deployment() {
    local STEP_NAME="register_deployment"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 11: Register deployment"

    if ! confirm "Submit register-deployment for $CHAIN?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts register-deployment "$CHAIN" \
        -m "$MNEMONIC" \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "register-deployment"
}

# =============================================================================
# Phase 3: Verification & Rewards
# =============================================================================

step_verify_gateway_registration() {
    local STEP_NAME="verify_gateway_registration"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 12: Verify Gateway registration"

    load_deployed_addresses

    if [[ -z "$ROUTER" ]]; then
        log_warn "Router address not found in config. Skipping verification."
        mark_step_done "$STEP_NAME"
        return
    fi

    log_info "Querying Router for chain info..."
    run_axelard q wasm contract-state smart "$ROUTER" \
        "{\"chain_info\": \"$CHAIN\"}" \
        --output json --node "$NODE" | jq . || {
        log_warn "Query failed. You may need to verify manually."
    }

    mark_step_done "$STEP_NAME"
}

step_verify_prover_authorized() {
    local STEP_NAME="verify_prover_authorized"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 13: Verify MultisigProver authorized on Multisig"

    load_deployed_addresses

    if [[ -z "$MULTISIG" || -z "$MULTISIG_PROVER" ]]; then
        log_warn "Multisig or MultisigProver address not found. Skipping verification."
        mark_step_done "$STEP_NAME"
        return
    fi

    log_info "Querying Multisig for caller authorization..."
    run_axelard q wasm contract-state smart "$MULTISIG" \
        "{\"is_caller_authorized\": {\"contract_address\": \"$MULTISIG_PROVER\", \"chain_name\": \"$CHAIN\"}}" \
        --output json --node "$NODE" | jq . || {
        log_warn "Query failed. You may need to verify manually."
    }

    mark_step_done "$STEP_NAME"
}

step_create_reward_pools() {
    local STEP_NAME="create_reward_pools"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 14: Create reward pools"

    local epoch_duration participation_threshold rewards_per_epoch
    epoch_duration=$(get_epoch_duration)
    participation_threshold=$(get_participation_threshold)
    rewards_per_epoch=$(get_rewards_per_epoch)

    log_info "Epoch duration:           $epoch_duration"
    log_info "Participation threshold:  $participation_threshold"
    log_info "Rewards per epoch:        $rewards_per_epoch"

    if ! confirm "Submit create-reward-pools for $CHAIN?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts create-reward-pools "$CHAIN" \
        --epochDuration "$epoch_duration" \
        --participationThreshold "$participation_threshold" \
        --rewardsPerEpoch "$rewards_per_epoch" \
        $GOVERNANCE_FLAG

    mark_step_done "$STEP_NAME"
    wait_for_proposal "create-reward-pools"
}

step_ampd_update_pause() {
    local STEP_NAME="ampd_update"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 15: Update ampd with Solana chain configuration"

    load_deployed_addresses

    local service_name
    service_name=$(get_service_name)

    echo ""
    echo "    Verifiers should add the following handlers to their ampd config:"
    echo ""
    echo "    [[handlers]]"
    echo "      - type: MultisigSigner"
    echo "        cosmwasm_contract: $MULTISIG"
    echo "        chain_name: $CHAIN"
    echo "      - type: SolanaMsgVerifier"
    echo "        chain_name: $CHAIN"
    echo "        cosmwasm_contract: $VOTING_VERIFIER"
    echo "        rpc_url: <SOLANA_RPC_URL>"
    echo "        gateway_address: $GATEWAY_CW"
    echo "      - type: SolanaVerifierSetVerifier"
    echo "        chain_name: $CHAIN"
    echo "        cosmwasm_contract: $VOTING_VERIFIER"
    echo "        rpc_url: <SOLANA_RPC_URL>"
    echo "        gateway_address: $GATEWAY_CW"
    echo ""
    echo "    Then register:"
    echo "      ampd register-public-key ed25519"
    echo "      ampd register-chain-support $service_name $CHAIN"
    echo ""

    if ! confirm "Have verifiers updated ampd and registered chain support?"; then
        log_warn "Script paused. Re-run to resume."
        exit 0
    fi

    mark_step_done "$STEP_NAME"
}

step_add_funds_to_reward_pools() {
    local STEP_NAME="add_funds_reward_pools"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 16: Add funds to reward pools"

    load_deployed_addresses

    local reward_amount
    reward_amount=$(get_reward_amount)

    if [[ -z "${REWARDS_WALLET:-}" ]]; then
        log_warn "REWARDS_WALLET not set. Printing commands for manual execution."
        echo ""
        echo "    A wallet with at least 2x ${reward_amount} is needed."
        echo "    Set REWARDS_WALLET to your axelard keyring name and re-run, or run manually:"
        echo ""
        echo "    axelard tx wasm execute $REWARDS \\"
        echo "      '{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }' \\"
        echo "      --amount $reward_amount --from <WALLET> --node $NODE"
        echo ""
        echo "    axelard tx wasm execute $REWARDS \\"
        echo "      '{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }' \\"
        echo "      --amount $reward_amount --from <WALLET> --node $NODE"
        echo ""
        echo "    Then verify: ts-node cosmwasm/query.ts rewards $CHAIN"
        echo ""

        if confirm "Have you funded the reward pools manually?"; then
            mark_step_done "$STEP_NAME"
        else
            log_warn "Script paused. Fund reward pools and re-run."
            exit 0
        fi
        return
    fi

    log_info "Reward amount: $reward_amount (per pool)"
    log_info "Wallet:        $REWARDS_WALLET"

    if ! confirm "Fund reward pools from $REWARDS_WALLET?"; then
        log_info "Skipping."
        return
    fi

    run_axelard tx wasm execute "$REWARDS" \
        "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" \
        --amount "$reward_amount" --from "$REWARDS_WALLET" --node "$NODE" \
        --gas auto --gas-adjustment 1.2 -y

    run_axelard tx wasm execute "$REWARDS" \
        "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" \
        --amount "$reward_amount" --from "$REWARDS_WALLET" --node "$NODE" \
        --gas auto --gas-adjustment 1.2 -y

    log_info "Verifying reward pools..."
    run_ts_node cosmwasm/query.ts rewards "$CHAIN" || true

    mark_step_done "$STEP_NAME"
}

step_query_active_verifiers() {
    local STEP_NAME="query_active_verifiers"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 17: Query active verifiers"

    load_deployed_addresses

    local service_name
    service_name=$(get_service_name)

    if [[ -z "$SERVICE_REGISTRY" ]]; then
        log_warn "ServiceRegistry address not found. Skipping."
        mark_step_done "$STEP_NAME"
        return
    fi

    run_axelard q wasm contract-state smart "$SERVICE_REGISTRY" \
        "{ \"active_verifiers\": { \"service_name\": \"$service_name\", \"chain_name\": \"$CHAIN\"} }" \
        --node "$NODE" --output json | jq . || {
        log_warn "Query failed."
    }

    mark_step_done "$STEP_NAME"
}

step_create_genesis_verifier_set() {
    local STEP_NAME="create_genesis_verifier_set"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 18: Create genesis verifier set"

    load_deployed_addresses

    if [[ -z "$MULTISIG_PROVER" ]]; then
        log_error "MultisigProver address not found in config."
        exit 1
    fi

    log_warn "This step can only run once sufficient verifiers have registered."

    if ! confirm "Create genesis verifier set?"; then
        log_info "Skipping."
        return
    fi

    run_axelard tx wasm execute "$MULTISIG_PROVER" '"update_verifier_set"' \
        --from "$PROVER_ADMIN" --gas auto --gas-adjustment 1.2 --node "$NODE" -y

    log_info "Querying current verifier set..."
    run_axelard q wasm contract-state smart "$MULTISIG_PROVER" \
        '"current_verifier_set"' --node "$NODE" --output json | jq . || true

    mark_step_done "$STEP_NAME"
}

# =============================================================================
# Summary
# =============================================================================

print_summary() {
    log_step "Deployment Summary"

    load_deployed_addresses

    echo ""
    echo "    Environment: ${ENV}"
    echo "    Chain:       ${CHAIN}"
    echo ""
    echo "    CosmWasm Contracts:"
    echo "      VotingVerifier:        ${VOTING_VERIFIER:-<not deployed>}"
    echo "      Gateway:               ${GATEWAY_CW:-<not deployed>}"
    echo "      MultisigProver:        ${MULTISIG_PROVER:-<not deployed>}"
    echo "      ItsSolanaTranslator:   $(jq_config '.axelar.contracts.ItsSolanaTranslator.address // "<not deployed>"')"
    echo ""
    echo "    Global Contracts:"
    echo "      Router:                ${ROUTER:-<not found>}"
    echo "      Multisig:              ${MULTISIG:-<not found>}"
    echo "      Rewards:               ${REWARDS:-<not found>}"
    echo "      ServiceRegistry:       ${SERVICE_REGISTRY:-<not found>}"
    echo ""
    echo "    Next: Return to solana/deploy.sh for Solana gateway initialization."
    echo "    See: releases/solana/2025-09-GMP-v1.0.0.md"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "CosmWasm Solana GMP Amplifier Deployment"
    log_info "Environment: ${ENV}"
    log_info "Chain:       ${CHAIN}"
    log_info "Versions:    VV=${VOTING_VERIFIER_VERSION} GW=${GATEWAY_VERSION} MP=${MULTISIG_PROVER_VERSION} ITS=${ITS_TRANSLATOR_VERSION}"
    echo ""

    check_prerequisites
    load_config_values
    build_init_addresses

    # Resolve secrets from 1Password (with env var fallback)
    log_step "Resolving secrets and addresses"
    resolve_mnemonic
    resolve_contract_admin
    resolve_prover_admin

    show_config_summary

    # Phase 1: Store Contracts
    step_store_voting_verifier
    step_store_gateway
    step_store_multisig_prover
    step_store_its_solana_translator
    step_instantiate_its_translator
    step_verify_its_translator_address

    # Phase 2: Configure & Deploy
    step_add_voting_verifier_config
    step_add_multisig_prover_config
    step_instantiate_chain_contracts
    step_save_deployed_contracts
    step_register_deployment

    # Phase 3: Verification & Rewards
    step_verify_gateway_registration
    step_verify_prover_authorized
    step_create_reward_pools
    step_ampd_update_pause
    step_add_funds_to_reward_pools
    step_query_active_verifiers
    step_create_genesis_verifier_set

    print_summary
    log_step "Done!"
}

show_config_summary() {
    log_step "Configuration Summary"
    echo ""
    echo "    Contract Admin:  $CONTRACT_ADMIN"
    echo "    Prover Admin:    $PROVER_ADMIN"
    echo "    Governance:      $GOVERNANCE_ADDRESS"
    echo "    Admin:           ${ADMIN_ADDRESS:-$GOVERNANCE_ADDRESS}"
    echo "    Coordinator:     ${COORDINATOR_ADDRESS:-<not set>}"
    echo "    Node:            $NODE"
    echo "    Governance mode: ${REQUIRES_GOVERNANCE}"
    echo ""

    if ! confirm "Continue with this configuration?"; then
        log_info "Aborting."
        exit 0
    fi
}

main "$@"
