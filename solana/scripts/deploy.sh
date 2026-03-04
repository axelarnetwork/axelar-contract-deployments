#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana GMP v1.0.0 Deployment Script
#
# Deploys and initializes Solana programs for Axelar.
# Reads ENV and CHAIN from solana/.env (same file used by solana/cli).
# Fetches upgrade-authority and operator keypairs from 1Password on-demand
# and cleans them up on exit.
#
# Prerequisites:
#   - Run setup-keys.sh first to generate keypairs and program IDs
#   - Publish a release to get a version number
#   - solana/.env must have ENV and CHAIN set
#   - 1Password CLI (op) must be authenticated
#
# Usage:
#   ./solana/scripts/deploy.sh --version 0.1.9
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOLANA_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEPLOYMENTS_DIR="$(cd "${SOLANA_DIR}/.." && pwd)"

# --- Logging ---
log_step()  { echo -e "\n\033[1;34m==> $1\033[0m"; }
log_info()  { echo "    $1"; }
log_warn()  { echo -e "    \033[1;33mWARNING: $1\033[0m"; }
log_error() { echo -e "\033[1;31mERROR: $1\033[0m"; }

confirm() {
    local prompt="${1:-Proceed?}"
    read -r -p "    ${prompt} [y/N] " response
    [[ "$response" =~ ^[Yy]$ ]]
}

# --- Source solana/.env ---
if [[ ! -f "${SOLANA_DIR}/.env" ]]; then
    log_error "solana/.env not found. Please create it with ENV and CHAIN."
    echo "Example:"
    echo "  ENV=stagenet"
    echo "  CHAIN=solana"
    exit 1
fi

# shellcheck source=/dev/null
source "${SOLANA_DIR}/.env"

# --- Validate ENV ---
case "${ENV:-}" in
    devnet-amplifier|stagenet|testnet|mainnet) ;;
    *)
        log_error "Invalid ENV='${ENV:-}'. Must be one of: devnet-amplifier, stagenet, testnet, mainnet"
        exit 1
        ;;
esac

if [[ -z "${CHAIN:-}" ]]; then
    log_error "CHAIN not set in solana/.env"
    exit 1
fi

# =============================================================================
# Environment-dependent configuration
# =============================================================================

get_cluster() {
    case "$ENV" in
        devnet-amplifier) echo "devnet" ;;
        stagenet|testnet) echo "testnet" ;;
        mainnet)          echo "mainnet-beta" ;;
    esac
}

get_minimum_rotation_delay() {
    case "$ENV" in
        devnet-amplifier) echo "0" ;;
        stagenet)         echo "300" ;;
        testnet)          echo "3600" ;;
        mainnet)          echo "86400" ;;
    esac
}

get_rpc_url() {
    local cluster
    cluster=$(get_cluster)
    case "$cluster" in
        devnet)       echo "https://api.devnet.solana.com" ;;
        testnet)      echo "https://api.testnet.solana.com" ;;
        mainnet-beta) echo "https://api.mainnet-beta.solana.com" ;;
    esac
}

get_explorer_name() {
    local cluster
    cluster=$(get_cluster)
    case "$cluster" in
        devnet)       echo "Solana Devnet Explorer" ;;
        testnet)      echo "Solana Testnet Explorer" ;;
        mainnet-beta) echo "Solana Mainnet Explorer" ;;
    esac
}

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

CLUSTER=$(get_cluster)
MINIMUM_ROTATION_DELAY=$(get_minimum_rotation_delay)
PREVIOUS_SIGNERS_RETENTION=15
CHAINS_INFO_FILE="${DEPLOYMENTS_DIR}/axelar-chains-config/info/${ENV}.json"
OP_VAULT=$(get_op_vault)
ENV_DISPLAY=$(get_env_display)

# Track temporary files for cleanup
TEMP_KEYPAIR_FILES=()

cleanup() {
    if [[ ${#TEMP_KEYPAIR_FILES[@]} -gt 0 ]]; then
        log_info "Cleaning up temporary keypair files..."
        for f in "${TEMP_KEYPAIR_FILES[@]}"; do
            if [[ -f "$f" ]]; then
                rm -f "$f"
            fi
        done
    fi
}
trap cleanup EXIT

# Programs to deploy: "display_name|cli_name|prefix|config_key"
DEPLOY_PROGRAMS=(
    "Gateway|gateway|gtw|AxelarGateway"
    "Gas Service|gas-service|gas|AxelarGasService"
    "Operators|operators|opr|AxelarOperators"
    "Memo|memo|mem|AxelarMemo"
    "ITS|its|its|InterchainTokenService"
)

# =============================================================================
# Flag parsing
# =============================================================================

usage() {
    echo "Usage: $0 --version <VERSION> [OPTIONS]"
    echo ""
    echo "Deploys and initializes Solana programs for Axelar."
    echo "Reads ENV, CHAIN, and keypair paths from solana/.env."
    echo ""
    echo "Prerequisites:"
    echo "  - Run setup-keys.sh first to generate keypairs"
    echo "  - Publish a release to get a version number"
    echo ""
    echo "Required:"
    echo "  --version <ver>            Semver (e.g. 0.1.9) or commit hash"
    echo ""
    echo "Optional:"
    echo "  -h, --help                 Show this help"
}

VERSION=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="$2"
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

if [[ -z "$VERSION" ]]; then
    log_error "--version is required"
    log_info "Find the latest version at: https://crates.io/crates/solana-axelar-gateway/versions"
    usage
    exit 1
fi

# Validate version format
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] && [[ ! "$VERSION" =~ ^[a-fA-F0-9]{7,}$ ]]; then
    log_error "Invalid version: $VERSION. Must be semver (e.g. 0.1.9) or commit hash (e.g. 12e6126)"
    exit 1
fi

# =============================================================================
# Utility functions
# =============================================================================

run_solana_cli() {
    "${SOLANA_DIR}/cli" "$@"
}

# =============================================================================
# Step functions
# =============================================================================

check_prerequisites() {
    log_step "Checking prerequisites"

    local missing=()
    command -v solana >/dev/null 2>&1 || missing+=("solana")
    command -v solana-keygen >/dev/null 2>&1 || missing+=("solana-keygen")
    command -v cargo >/dev/null 2>&1 || missing+=("cargo")
    command -v jq >/dev/null 2>&1 || missing+=("jq")
    command -v op >/dev/null 2>&1 || missing+=("op (1Password CLI)")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi
    log_info "All tools available"

    if [[ ! -f "$CHAINS_INFO_FILE" ]]; then
        log_error "Chains info file not found: $CHAINS_INFO_FILE"
        exit 1
    fi
    log_info "All files verified"
}

# Fetch a document from 1Password by title, save to a temp file, return the path.
# The file is tracked for cleanup on exit.
fetch_keypair_from_op() {
    local title="$1"
    mkdir -p "${SOLANA_DIR}/deployments"
    # [Stagenet] Gas Service: Solana → stagenet-gas-service-solana.json
    local sanitized
    sanitized=$(echo "$title" | tr '[:upper:]' '[:lower:]' | sed 's/[][]//g; s/://g; s/  */-/g; s/^-//; s/-$//')
    local output_path="${SOLANA_DIR}/deployments/${sanitized}.json"

    log_info "Fetching '${title}' from 1Password..." >&2
    op document get "$title" --vault "$OP_VAULT" --out-file "$output_path" --force >/dev/null 2>&1 || {
        log_error "Failed to fetch '${title}' from 1Password vault '${OP_VAULT}'"
        log_info "Ensure the document exists and you are authenticated (op signin)."
        log_info "The item must be a Document type, not a Secure Note."
        exit 1
    }

    TEMP_KEYPAIR_FILES+=("$output_path")
    echo "$output_path"
}

resolve_keypairs() {
    log_step "Fetching keypairs from 1Password"

    # Fetch upgrade authority
    UPGRADE_AUTHORITY_KEYPAIR_PATH=$(fetch_keypair_from_op "[${ENV_DISPLAY}] Upgrade Authority: Solana")
    UPGRADE_AUTHORITY_PUBKEY=$(solana-keygen pubkey "$UPGRADE_AUTHORITY_KEYPAIR_PATH")
    log_info "Upgrade authority: $UPGRADE_AUTHORITY_PUBKEY"

    # Fetch operator
    OPERATOR_KEYPAIR_PATH=$(fetch_keypair_from_op "[${ENV_DISPLAY}] Operator: Solana")
    OPERATOR_PUBKEY=$(solana-keygen pubkey "$OPERATOR_KEYPAIR_PATH")
    log_info "Operator: $OPERATOR_PUBKEY"

    # Fetch program keypairs
    local program_names=("Gateway" "Gas Service" "Operators" "Memo" "ITS")
    local program_prefixes=("gtw" "gas" "opr" "mem" "its")

    for i in "${!program_names[@]}"; do
        local name="${program_names[$i]}"
        local prefix="${program_prefixes[$i]}"
        local title="[${ENV_DISPLAY}] ${name}: Solana"
        local keypair_path
        keypair_path=$(fetch_keypair_from_op "$title")

        local pubkey
        pubkey=$(solana-keygen pubkey "$keypair_path")
        log_info "${name}: ${pubkey}"

        # Store path in an associative-style variable for deploy_programs to use
        eval "PROGRAM_KEYPAIR_${prefix}=\"${keypair_path}\""
    done
}

show_balance() {
    log_step "Checking SOL balance"
    solana config set --url "$CLUSTER" 2>&1 | sed 's/^/    /'
    solana config set --keypair "$UPGRADE_AUTHORITY_KEYPAIR_PATH" 2>&1 | sed 's/^/    /'
    local balance
    balance=$(solana balance 2>&1)
    log_info "Balance: $balance"

    local amount
    amount=$(echo "$balance" | awk '{print $1}')
    if command -v bc >/dev/null 2>&1; then
        if (( $(echo "$amount < 2" | bc -l 2>/dev/null || echo 0) )); then
            log_warn "Low balance! Deployments require significant SOL."
            if [[ "$ENV" != "mainnet" ]]; then
                log_info "Run: solana airdrop 5"
            fi
        fi
    fi
}

add_chain_config() {
    log_step "Adding Solana chain config to ${ENV}.json"

    local existing
    existing=$(jq -r ".chains[\"${CHAIN}\"] // empty" "$CHAINS_INFO_FILE")

    if [[ -n "$existing" ]]; then
        log_info "Solana chain entry '${CHAIN}' already exists in ${ENV}.json. Skipping."
        return
    fi

    local rpc_url
    rpc_url=$(get_rpc_url)
    local explorer_name
    explorer_name=$(get_explorer_name)

    local tmp_file="${CHAINS_INFO_FILE}.tmp"
    jq --arg chain "$CHAIN" \
       --arg rpc "$rpc_url" \
       --arg explorer_name "$explorer_name" \
       --arg cluster "$CLUSTER" \
    '.chains[$chain] = {
        "name": "Solana",
        "axelarId": $chain,
        "rpc": $rpc,
        "chainType": "svm",
        "decimals": 9,
        "finality": "31",
        "approxFinalityWaitTime": 1,
        "tokenSymbol": "SOL",
        "explorer": {
            "name": $explorer_name,
            "url": ("https://explorer.solana.com/?cluster=" + $cluster)
        },
        "contracts": {}
    }' "$CHAINS_INFO_FILE" > "$tmp_file" && mv "$tmp_file" "$CHAINS_INFO_FILE"

    log_info "Added '${CHAIN}' chain entry to ${ENV}.json"
}

deploy_programs() {
    log_step "Deploying Solana programs"
    log_warn "Governance deployment is not yet supported by this script."
    echo ""

    for entry in "${DEPLOY_PROGRAMS[@]}"; do
        IFS='|' read -r name cli_name prefix config_key <<< "$entry"

        local keypair_file
        eval "keypair_file=\${PROGRAM_KEYPAIR_${prefix}}"
        local pubkey
        pubkey=$(solana-keygen pubkey "$keypair_file")

        # Check if program is already deployed in config
        local existing_addr
        existing_addr=$(jq -r ".chains[\"${CHAIN}\"].contracts[\"${config_key}\"].address // empty" "$CHAINS_INFO_FILE")
        local existing_version
        existing_version=$(jq -r ".chains[\"${CHAIN}\"].contracts[\"${config_key}\"].version // empty" "$CHAINS_INFO_FILE")

        log_step "Deploy ${name}"
        log_info "Program:           ${cli_name}"
        log_info "PDA:               ${pubkey}"
        log_info "Version:           ${VERSION}"
        log_info "Upgrade Authority: ${UPGRADE_AUTHORITY_PUBKEY}"

        if [[ -n "$existing_addr" ]]; then
            log_warn "${name} already deployed at ${existing_addr} (version: ${existing_version:-unknown})"
            if ! confirm "Redeploy ${name}?"; then
                log_info "Skipping ${name}"
                continue
            fi
        else
            if ! confirm "Deploy ${name}?"; then
                log_info "Skipping ${name}"
                continue
            fi
        fi

        run_solana_cli deploy \
            --program "$cli_name" \
            --version "$VERSION" \
            --program-keypair "$keypair_file" \
            --upgrade-authority "$UPGRADE_AUTHORITY_KEYPAIR_PATH" \
            -y

        log_info "${name} deployed successfully"
    done
}

cosmwasm_pause() {
    log_step "CosmWasm Deployment Required"
    echo ""
    echo "    All Solana programs have been deployed."
    echo "    Deploy the CosmWasm Amplifier contracts before continuing."
    echo ""
    echo "    Run:  ./solana/scripts/deploy-axelar.sh"
    echo ""
    echo "    This deploys VotingVerifier, Gateway, MultisigProver, and"
    echo "    ItsSolanaTranslator via the Coordinator contract."
    echo ""

    if ! confirm "Have you deployed the CosmWasm contracts? Press y to continue."; then
        log_error "Aborting. Run ./solana/scripts/deploy-axelar.sh and then re-run this script."
        exit 1
    fi
}

verify_cosmwasm_config() {
    log_step "Verifying CosmWasm contract config"

    local prover_address
    prover_address=$(jq -r ".axelar.contracts.MultisigProver[\"${CHAIN}\"].address // empty" "$CHAINS_INFO_FILE")

    if [[ -z "$prover_address" ]]; then
        log_error "MultisigProver address for '${CHAIN}' not found in ${ENV}.json"
        log_info "Ensure CosmWasm contracts are registered in the config before continuing."
        exit 1
    fi
    log_info "MultisigProver: $prover_address"

    local domain_separator
    domain_separator=$(jq -r ".axelar.contracts.MultisigProver[\"${CHAIN}\"].domainSeparator // empty" "$CHAINS_INFO_FILE")

    if [[ -z "$domain_separator" ]]; then
        log_error "Domain separator for '${CHAIN}' not found in ${ENV}.json"
        exit 1
    fi
    log_info "Domain separator: $domain_separator"
}

initialize_programs() {
    log_step "Initializing Solana programs"

    if ! confirm "Proceed with program initialization?"; then
        log_info "Skipping initialization"
        return
    fi

    # 1. Initialize Gateway
    log_step "Initializing Gateway"
    log_info "minimum-rotation-delay:     ${MINIMUM_ROTATION_DELAY}"
    log_info "previous-signers-retention: ${PREVIOUS_SIGNERS_RETENTION}"
    log_info "operator:                   ${OPERATOR_PUBKEY}"

    if confirm "Initialize Gateway?"; then
        run_solana_cli send gateway init \
            --previous-signers-retention "$PREVIOUS_SIGNERS_RETENTION" \
            --minimum-rotation-delay "$MINIMUM_ROTATION_DELAY" \
            --operator "$OPERATOR_PUBKEY"

        log_info "Gateway initialized"
    else
        log_info "Skipping Gateway"
    fi

    # 2. Initialize Operators
    log_step "Initializing Operators"
    log_info "owner: ${UPGRADE_AUTHORITY_PUBKEY}"

    if confirm "Initialize Operators?"; then
        run_solana_cli send operators init \
            --owner "$UPGRADE_AUTHORITY_PUBKEY"

        log_info "Operators initialized"
    else
        log_info "Skipping Operators"
    fi

    # 3. Add Operator
    log_step "Adding operator"
    log_info "operator: ${OPERATOR_PUBKEY}"

    if confirm "Add operator?"; then
        run_solana_cli send operators add-operator \
            --operator "$OPERATOR_PUBKEY"

        log_info "Operator added"
    else
        log_info "Skipping add operator"
    fi

    # 4. Initialize Gas Service
    log_step "Initializing Gas Service"
    log_info "operator: ${OPERATOR_PUBKEY}"
    log_info "signer:   ${OPERATOR_KEYPAIR_PATH}"

    if confirm "Initialize Gas Service?"; then
        run_solana_cli send --signer-keys "$OPERATOR_KEYPAIR_PATH" gas-service init \
            --operator "$OPERATOR_PUBKEY"

        log_info "Gas Service initialized"
    else
        log_info "Skipping Gas Service"
    fi

    # 5. Initialize ITS
    log_step "Initializing ITS"
    local its_hub_address
    its_hub_address=$(jq -r '.axelar.contracts.InterchainTokenService.address // empty' "$CHAINS_INFO_FILE")
    if [[ -z "$its_hub_address" ]]; then
        log_error "InterchainTokenService address not found in config."
        log_info "Ensure the ITS Hub contract is deployed before initializing ITS."
        exit 1
    fi
    log_info "operator:         ${OPERATOR_PUBKEY}"
    log_info "chain-name:       ${CHAIN}"
    log_info "its-hub-address:  ${its_hub_address}"

    if confirm "Initialize ITS?"; then
        run_solana_cli send -s "$OPERATOR_KEYPAIR_PATH" its init \
            --operator "$OPERATOR_PUBKEY" \
            --chain-name "$CHAIN" \
            --its-hub-address "$its_hub_address"

        log_info "ITS initialized"
    else
        log_info "Skipping ITS"
    fi
}

print_summary() {
    log_step "Deployment Summary"
    echo ""
    echo "    Environment: ${ENV}"
    echo "    Chain:       ${CHAIN}"
    echo "    Cluster:     ${CLUSTER}"
    echo "    Version:     ${VERSION}"
    echo ""
    echo "    Programs deployed:"
    for entry in "${DEPLOY_PROGRAMS[@]}"; do
        IFS='|' read -r name cli_name prefix _config_key <<< "$entry"
        local keypair_file
        eval "keypair_file=\${PROGRAM_KEYPAIR_${prefix}:-}"
        if [[ -n "$keypair_file" ]] && [[ -f "$keypair_file" ]]; then
            local pubkey
            pubkey=$(solana-keygen pubkey "$keypair_file")
            echo "      ${name}: ${pubkey}"
        fi
    done
    echo ""
    echo "    Upgrade Authority: ${UPGRADE_AUTHORITY_PUBKEY}"
    echo "    Operator:          ${OPERATOR_PUBKEY}"
    echo ""
    echo "    Next: run ./solana/scripts/setup-its.sh to register ITS on the hub"
    echo "    Then: run ./solana/scripts/checklist.sh for post-deployment verification"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana Deployment Script"
    log_info "Environment: ${ENV}"
    log_info "Chain:       ${CHAIN}"
    log_info "Cluster:     ${CLUSTER}"
    log_info "Version:     ${VERSION}"
    echo ""

    check_prerequisites
    resolve_keypairs
    show_balance
    add_chain_config
    deploy_programs
    cosmwasm_pause
    verify_cosmwasm_config
    initialize_programs
    print_summary

    log_step "Done!"
}

main "$@"
