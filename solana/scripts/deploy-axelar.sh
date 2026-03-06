#!/bin/bash
set -euo pipefail

# =============================================================================
# CosmWasm Solana GMP Amplifier Deployment Script
#
# Deploys CosmWasm Amplifier contracts for Solana GMP connection.
# Automates the steps in releases/cosmwasm/2025-09-Solana-GMP-v1.0.0.md.
#
# Run this script AFTER solana/scripts/deploy.sh deploys Solana programs.
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
#   ENV=stagenet CHAIN=solana ./solana/scripts/deploy-axelar.sh
#   ./solana/scripts/deploy-axelar.sh --reset                    # start fresh
#   ./solana/scripts/deploy-axelar.sh --version-vv 2.1.0         # override version
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOLANA_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEPLOYMENTS_DIR="$(cd "${SOLANA_DIR}/.." && pwd)"

# --- Logging (same as solana/scripts/deploy.sh) ---
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
if [[ -f "${SOLANA_DIR}/.env" ]]; then
    # shellcheck source=/dev/null
    source "${SOLANA_DIR}/.env"
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
    echo "${SALT:-$CHAIN}"
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
    echo "  --version-vv <ver>       VotingVerifier version   (default: 2.0.1)"
    echo "  --version-gw <ver>       Gateway version          (default: 1.1.1)"
    echo "  --version-mp <ver>       MultisigProver version   (default: 1.2.0)"
    echo "  --version-its <ver>      ItsSolanaTranslator ver  (default: 1.0.0)"
    echo "  --salt <salt>            Salt for instantiate2   (default: \$CHAIN)"
    echo "  -h, --help               Show this help"
    echo ""
    echo "Environment variables:"
    echo "  ENV              Required: devnet-amplifier, stagenet, testnet, mainnet"
    echo "  CHAIN            Required: solana chain name (e.g. solana)"
    echo "  MNEMONIC         Fallback if 1Password unavailable"
    echo "  NODE             Axelar RPC URL (reads from config if unset)"
}

VOTING_VERIFIER_VERSION="2.0.1"
GATEWAY_VERSION="1.1.1"
MULTISIG_PROVER_VERSION="1.2.0"
ITS_TRANSLATOR_VERSION="1.0.0"
SALT=""
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
        --salt)
            SALT="$2"
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

mkdir -p "${SOLANA_DIR}/deployments"
STATE_FILE="${SOLANA_DIR}/deployments/.deploy-axelar-state-${ENV}-${CHAIN}"

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

    # Try root .env file
    if [[ -z "${MNEMONIC:-}" ]] && [[ -f "${DEPLOYMENTS_DIR}/.env" ]]; then
        # shellcheck source=/dev/null
        source "${DEPLOYMENTS_DIR}/.env"
        if [[ -n "${MNEMONIC:-}" ]]; then
            log_info "Mnemonic loaded from root .env"
            return
        fi
    fi

    if [[ -z "${MNEMONIC:-}" ]]; then
        log_error "MNEMONIC not set. Set it via environment, root .env, or configure 1Password."
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

# Find an existing code ID for a contract+version by checking other chains.
# Usage: find_existing_code_id "VotingVerifier" "2.0.1"
# Prints the code ID if found, empty string otherwise.
find_existing_code_id() {
    local contract="$1"
    local version="$2"
    jq -r --arg v "$version" \
        ".axelar.contracts.${contract} | to_entries[] | select(.value | type == \"object\" and .version == \$v and .codeId) | .value.codeId" \
        "$CHAINS_INFO_FILE" | head -1
}

# Ensure the code ID for a chain-scoped contract matches the expected value.
# If it differs, overwrite it in the config.
ensure_code_id() {
    local contract="$1"
    local chain="$2"
    local expected_code_id="$3"
    local current
    current=$(jq_config ".axelar.contracts.${contract}[\"${chain}\"].codeId // empty")
    if [[ "$current" != "$expected_code_id" ]]; then
        log_info "Updating ${contract}[${chain}].codeId: ${current:-<empty>} -> $expected_code_id"
        local tmp_file="${CHAINS_INFO_FILE}.tmp"
        jq --arg chain "$chain" --argjson cid "$expected_code_id" \
            ".axelar.contracts.${contract}[\$chain].codeId = \$cid" \
            "$CHAINS_INFO_FILE" > "$tmp_file" && mv "$tmp_file" "$CHAINS_INFO_FILE"
    fi
}

load_config_values() {
    log_step "Loading configuration from ${ENV}.json"

    GOVERNANCE_ADDRESS=$(jq_config '.axelar.governanceAddress')
    COORDINATOR_ADDRESS=$(jq_config '.axelar.contracts.Coordinator.address // empty')
    CHAIN_ID=$(jq_config '.axelar.chainId // empty')

    NODE="${NODE:-$(jq_config '.axelar.rpc // empty')}"
    if [[ -z "$NODE" ]]; then
        log_error "NODE not set and axelar.rpc not found in config"
        exit 1
    fi

    log_info "Governance:  $GOVERNANCE_ADDRESS"
    log_info "Coordinator: ${COORDINATOR_ADDRESS:-<not set>}"
    log_info "Chain ID:    $CHAIN_ID"
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
        log_info "Run solana/scripts/deploy.sh first to create the chain entry."
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

    local existing_code_id
    existing_code_id=$(find_existing_code_id "VotingVerifier" "$VOTING_VERIFIER_VERSION")
    if [[ -n "$existing_code_id" ]]; then
        log_info "Found existing codeId=$existing_code_id for VotingVerifier v${VOTING_VERIFIER_VERSION}"
        if confirm "Reuse this code ID? (n to store fresh code)"; then
            ensure_code_id "VotingVerifier" "$CHAIN" "$existing_code_id"
            mark_step_done "$STEP_NAME"
            return
        fi
    fi

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

    wait_for_proposal "VotingVerifier store-code"
    mark_step_done "$STEP_NAME"
}

step_store_gateway() {
    local STEP_NAME="store_gateway"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 2: Store Gateway (v${GATEWAY_VERSION})"

    local existing_code_id
    existing_code_id=$(find_existing_code_id "Gateway" "$GATEWAY_VERSION")
    if [[ -n "$existing_code_id" ]]; then
        log_info "Found existing codeId=$existing_code_id for Gateway v${GATEWAY_VERSION}"
        if confirm "Reuse this code ID? (n to store fresh code)"; then
            ensure_code_id "Gateway" "$CHAIN" "$existing_code_id"
            mark_step_done "$STEP_NAME"
            return
        fi
    fi

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

    wait_for_proposal "Gateway store-code"
    mark_step_done "$STEP_NAME"
}

step_store_multisig_prover() {
    local STEP_NAME="store_multisig_prover"
    if is_step_done "$STEP_NAME"; then
        log_info "[$STEP_NAME] Already completed. Skipping."
        return
    fi

    log_step "Step 3: Store MultisigProver (v${MULTISIG_PROVER_VERSION})"

    local existing_code_id
    existing_code_id=$(find_existing_code_id "MultisigProver" "$MULTISIG_PROVER_VERSION")
    if [[ -n "$existing_code_id" ]]; then
        log_info "Found existing codeId=$existing_code_id for MultisigProver v${MULTISIG_PROVER_VERSION}"
        if confirm "Reuse this code ID? (n to store fresh code)"; then
            ensure_code_id "MultisigProver" "$CHAIN" "$existing_code_id"
            mark_step_done "$STEP_NAME"
            return
        fi
    fi

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

    wait_for_proposal "MultisigProver store-code"
    mark_step_done "$STEP_NAME"
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
        if confirm "Use this already deployed contract? (n to store fresh code)"; then
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

    wait_for_proposal "ItsSolanaTranslator store-code"
    mark_step_done "$STEP_NAME"
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
    CHAIN= run_ts_node cosmwasm/contract.ts instantiate \
        -c ItsSolanaTranslator \
        -m "$MNEMONIC" \
        --fetchCodeId \
        $GOVERNANCE_FLAG

    wait_for_proposal "ItsSolanaTranslator instantiate"
    mark_step_done "$STEP_NAME"
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

    # Query on-chain to find the address by code ID
    local code_id
    code_id=$(jq_config '.axelar.contracts.ItsSolanaTranslator.codeId // empty')
    if [[ -z "$code_id" ]]; then
        log_warn "ItsSolanaTranslator codeId not found in config. Cannot query on-chain."
        log_info "Update axelar.contracts.ItsSolanaTranslator.codeId in ${ENV}.json and re-run."
        if ! confirm "Has the codeId been updated in the config?"; then
            log_warn "Script paused. Update the config and re-run."
            exit 0
        fi
        code_id=$(jq_config '.axelar.contracts.ItsSolanaTranslator.codeId // empty')
    fi

    if [[ -n "$code_id" ]]; then
        log_info "Querying contracts for code ID: $code_id"
        local query_result
        query_result=$(axelard q wasm list-contract-by-code "$code_id" --node "$NODE" --output json 2>/dev/null) || true

        local contract_addr
        contract_addr=$(echo "$query_result" | jq -r '.contracts[-1] // empty' 2>/dev/null)

        if [[ -n "$contract_addr" ]]; then
            log_info "Found ItsSolanaTranslator address: $contract_addr"

            if confirm "Save this address to ${ENV}.json?"; then
                local tmp_file="${CHAINS_INFO_FILE}.tmp"
                jq --arg addr "$contract_addr" \
                    '.axelar.contracts.ItsSolanaTranslator.address = $addr' \
                    "$CHAINS_INFO_FILE" > "$tmp_file" && mv "$tmp_file" "$CHAINS_INFO_FILE"
                log_info "Updated ItsSolanaTranslator address in config."
            fi
        else
            log_warn "No contracts found for code ID $code_id. Has the proposal passed?"
            if ! confirm "Continue anyway?"; then
                log_warn "Script paused. Re-run after the proposal has passed."
                exit 0
            fi
        fi
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

    local existing_svc
    existing_svc=$(jq_config ".axelar.contracts.VotingVerifier[\"${CHAIN}\"].serviceName // empty")
    if [[ -n "$existing_svc" ]]; then
        log_info "VotingVerifier[$CHAIN] already configured (serviceName=$existing_svc). Skipping."
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
    '.axelar.contracts.VotingVerifier[$chain] += {
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

    local existing_svc
    existing_svc=$(jq_config ".axelar.contracts.MultisigProver[\"${CHAIN}\"].serviceName // empty")
    if [[ -n "$existing_svc" ]]; then
        log_info "MultisigProver[$CHAIN] already configured (serviceName=$existing_svc). Skipping."
        mark_step_done "$STEP_NAME"
        return
    fi

    local service_name signing_threshold
    service_name=$(get_service_name)
    signing_threshold=$(get_signing_threshold)

    log_info "governanceAddress:  $GOVERNANCE_ADDRESS"
    log_info "adminAddress:       $PROVER_ADMIN"
    log_info "signingThreshold:   $signing_threshold"
    log_info "serviceName:        $service_name"

    if ! confirm "Add MultisigProver config for $CHAIN?"; then
        log_info "Skipping."
        return
    fi

    local tmp_file="${CHAINS_INFO_FILE}.tmp"
    jq --arg chain "$CHAIN" \
       --arg gov "$GOVERNANCE_ADDRESS" \
       --arg admin "$PROVER_ADMIN" \
       --argjson thresh "$signing_threshold" \
       --arg svc "$service_name" \
    '.axelar.contracts.MultisigProver[$chain] += {
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

    local vv_code_id gw_code_id mp_code_id
    vv_code_id=$(jq_config ".axelar.contracts.VotingVerifier[\"${CHAIN}\"].codeId // empty")
    gw_code_id=$(jq_config ".axelar.contracts.Gateway[\"${CHAIN}\"].codeId // empty")
    mp_code_id=$(jq_config ".axelar.contracts.MultisigProver[\"${CHAIN}\"].codeId // empty")

    log_info "Chain:          $CHAIN"
    log_info "Salt:           $salt"
    log_info "Contract Admin: $CONTRACT_ADMIN"
    log_info "Code IDs:       VV=$vv_code_id  GW=$gw_code_id  MP=$mp_code_id"

    if [[ -z "$vv_code_id" || -z "$gw_code_id" || -z "$mp_code_id" ]]; then
        log_error "Missing code IDs in config. Ensure store-code steps completed successfully."
        return
    fi

    if ! confirm "Submit instantiate-chain-contracts?"; then
        log_info "Skipping."
        return
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts instantiate-chain-contracts \
        -n "$CHAIN" \
        -s "$salt" \
        --admin "$CONTRACT_ADMIN" \
        -m "$MNEMONIC" \
        $GOVERNANCE_FLAG

    wait_for_proposal "instantiate-chain-contracts"
    mark_step_done "$STEP_NAME"
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

    wait_for_proposal "register-deployment"
    mark_step_done "$STEP_NAME"
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
    axelard q wasm contract-state smart "$ROUTER" \
        '{"chain_info":"'"$CHAIN"'"}' \
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
    axelard q wasm contract-state smart "$MULTISIG" \
        '{"is_caller_authorized":{"contract_address":"'"$MULTISIG_PROVER"'","chain_name":"'"$CHAIN"'"}}' \
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
        -m "$MNEMONIC" \
        --epochDuration "$epoch_duration" \
        --participationThreshold "$participation_threshold" \
        --rewardsPerEpoch "$rewards_per_epoch" \
        $GOVERNANCE_FLAG

    wait_for_proposal "create-reward-pools"
    mark_step_done "$STEP_NAME"
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
    local domain_separator
    domain_separator=$(jq_config ".axelar.contracts.MultisigProver[\"${CHAIN}\"].domainSeparator // empty")

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
    echo "        domain_separator: $domain_separator"
    echo "      - type: SolanaVerifierSetVerifier"
    echo "        chain_name: $CHAIN"
    echo "        cosmwasm_contract: $VOTING_VERIFIER"
    echo "        rpc_url: <SOLANA_RPC_URL>"
    echo "        gateway_address: $GATEWAY_CW"
    echo "        domain_separator: $domain_separator"
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

    log_info "Reward amount: $reward_amount (per pool, 2 pools total)"
    log_info "Any funded wallet can perform this step."
    echo ""
    echo "    Run the following commands (replace <FROM> and <KEYRING_BACKEND>):"
    echo ""
    echo "    # Fund Multisig reward pool"
    echo "    axelard tx wasm execute $REWARDS \\"
    echo "      '{\"add_rewards\":{\"pool_id\":{\"chain_name\":\"$CHAIN\",\"contract\":\"$MULTISIG\"}}}' \\"
    echo "      --amount $reward_amount --from <FROM> --keyring-backend <KEYRING_BACKEND> \\"
    echo "      --chain-id $CHAIN_ID --node $NODE --gas auto --gas-adjustment 1.5 -y"
    echo ""
    echo "    # Fund VotingVerifier reward pool"
    echo "    axelard tx wasm execute $REWARDS \\"
    echo "      '{\"add_rewards\":{\"pool_id\":{\"chain_name\":\"$CHAIN\",\"contract\":\"$VOTING_VERIFIER\"}}}' \\"
    echo "      --amount $reward_amount --from <FROM> --keyring-backend <KEYRING_BACKEND> \\"
    echo "      --chain-id $CHAIN_ID --node $NODE --gas auto --gas-adjustment 1.5 -y"
    echo ""

    if ! confirm "Have the reward pools been funded?"; then
        log_warn "Script paused. Re-run to resume."
        exit 0
    fi

    # Extract expected amount (numeric part only, e.g. "1000000" from "1000000uaxl")
    local expected_amount
    expected_amount=$(echo "$reward_amount" | sed 's/[^0-9]//g')

    log_info "Verifying reward pools..."
    local verified=true

    for pool_name in "Multisig:$MULTISIG" "VotingVerifier:$VOTING_VERIFIER"; do
        local label="${pool_name%%:*}"
        local contract="${pool_name#*:}"

        log_info "$label reward pool:"
        local result
        result=$(axelard q wasm contract-state smart "$REWARDS" \
            '{"rewards_pool":{"pool_id":{"chain_name":"'"$CHAIN"'","contract":"'"$contract"'"}}}' \
            --node "$NODE" --output json 2>/dev/null) || result="{}"

        echo "$result" | jq '.data' 2>/dev/null || echo "$result"

        local balance
        balance=$(echo "$result" | jq -r '.data.balance // "0"' 2>/dev/null) || balance="0"

        if [[ "$balance" -lt "$expected_amount" ]]; then
            log_error "$label reward pool balance ($balance) is less than expected ($expected_amount)"
            verified=false
        fi
    done

    if [[ "$verified" != "true" ]]; then
        log_error "Reward pool verification failed. Not marking step as done."
        log_info "Fund the pools and re-run the script."
        exit 0
    fi

    log_info "Reward pools verified successfully."
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

    axelard q wasm contract-state smart "$SERVICE_REGISTRY" \
        '{"active_verifiers":{"service_name":"'"$service_name"'","chain_name":"'"$CHAIN"'"}}' \
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

    local mp_admin mp_governance
    mp_admin=$(jq_config ".axelar.contracts.MultisigProver[\"${CHAIN}\"].adminAddress // empty")
    mp_governance=$(jq_config ".axelar.contracts.MultisigProver[\"${CHAIN}\"].governanceAddress // empty")

    log_warn "This step can only run once sufficient verifiers have registered."
    echo ""
    echo "    This step requires the MultisigProver admin or governance address:"
    echo "      admin:      ${mp_admin:-<not set>}"
    echo "      governance:  ${mp_governance:-<not set>}"
    echo ""
    echo "    Run the following command (replace <FROM> and <KEYRING_BACKEND>):"
    echo ""
    echo "    axelard tx wasm execute $MULTISIG_PROVER '\"update_verifier_set\"' \\"
    echo "      --from <FROM> --keyring-backend <KEYRING_BACKEND> \\"
    echo "      --chain-id $CHAIN_ID --gas auto --gas-adjustment 1.5 --node $NODE -y"
    echo ""

    if ! confirm "Has the genesis verifier set been created?"; then
        log_warn "Script paused. Re-run to resume."
        exit 0
    fi

    log_info "Querying current verifier set..."
    axelard q wasm contract-state smart "$MULTISIG_PROVER" \
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
    echo "    Next: Return to solana/scripts/deploy.sh for Solana gateway initialization."
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
