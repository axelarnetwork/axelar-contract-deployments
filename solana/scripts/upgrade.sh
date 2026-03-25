#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana Program Upgrade Script
#
# Upgrades deployed Solana programs to a new version.
# Reads ENV and CHAIN from solana/.env.
# Fetches upgrade-authority keypair from 1Password on-demand
# and cleans it up on exit.
#
# Prerequisites:
#   - Programs must already be deployed (run deploy.sh first)
#   - solana/.env must have ENV and CHAIN set
#   - 1Password CLI (op) must be authenticated
#
# Usage:
#   ./solana/scripts/upgrade.sh --version 1.0.0
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
    echo "  ENV=devnet-amplifier"
    echo "  CHAIN=solana-18"
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
        devnet-amplifier)         echo "devnet" ;;
        stagenet|testnet)         echo "testnet" ;;
        mainnet)                  echo "mainnet-beta" ;;
    esac
}

get_op_vault() {
    case "$ENV" in
        devnet-amplifier) echo "Devnet - Axelar Externally Owned Accounts" ;;
        stagenet)         echo "Stagenet - Axelar Externally Owned Accounts" ;;
        testnet)          echo "Testnet - Axelar Externally Owned Accounts" ;;
        mainnet)          echo "Mainnet - Axelar Externally Owned Accounts";;
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
CHAINS_INFO_FILE="${DEPLOYMENTS_DIR}/axelar-chains-config/info/${ENV}.json"
OP_VAULT=$(get_op_vault)
ENV_DISPLAY=$(get_env_display)

# Export so child processes (cli wrapper → cargo run → CLI binary) pick up CLUSTER via clap env
export CLUSTER

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

# Programs to upgrade: "display_name|cli_name"
UPGRADE_PROGRAMS=(
    "Gateway|gateway"
    "Gas Service|gas-service"
    "Operators|operators"
    "Memo|memo"
    "ITS|its"
)

# =============================================================================
# Flag parsing
# =============================================================================

usage() {
    echo "Usage: $0 --version <VERSION> [OPTIONS]"
    echo ""
    echo "Upgrades deployed Solana programs to a new version."
    echo "Reads ENV and CHAIN from solana/.env."
    echo ""
    echo "Required:"
    echo "  --version <ver>            Semver (e.g. 1.0.0) or commit hash"
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
    usage
    exit 1
fi

# Validate version format
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] && [[ ! "$VERSION" =~ ^[a-fA-F0-9]{7,}$ ]]; then
    log_error "Invalid version: $VERSION. Must be semver (e.g. 1.0.0) or commit hash (e.g. 12e6126)"
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
    log_info "Chains info file: $CHAINS_INFO_FILE"
}

# Fetch a document from 1Password by title, save to a temp file, return the path.
fetch_keypair_from_op() {
    local title="$1"
    mkdir -p "${SOLANA_DIR}/deployments"
    local sanitized
    sanitized=$(echo "$title" | tr '[:upper:]' '[:lower:]' | sed 's/[][]//g; s/://g; s/  */-/g; s/^-//; s/-$//')
    local output_path="${SOLANA_DIR}/deployments/${sanitized}.json"

    log_info "Fetching '${title}' from 1Password..." >&2
    local op_stderr
    op_stderr=$(op document get "$title" --vault "$OP_VAULT" --out-file "$output_path" --force 2>&1 >/dev/null) || {
        log_error "Failed to fetch '${title}' from 1Password vault '${OP_VAULT}'" >&2
        log_info "op error: ${op_stderr}" >&2
        log_info "Ensure the document exists and you are authenticated (op signin)." >&2
        log_info "The item must be a Document type, not a Secure Note." >&2
        exit 1
    }

    TEMP_KEYPAIR_FILES+=("$output_path")
    echo "$output_path"
}

resolve_upgrade_authority() {
    log_step "Fetching upgrade authority keypair from 1Password"

    UPGRADE_AUTHORITY_KEYPAIR_PATH=$(fetch_keypair_from_op "[${ENV_DISPLAY}] Upgrade Authority: Solana")
    UPGRADE_AUTHORITY_PUBKEY=$(solana-keygen pubkey "$UPGRADE_AUTHORITY_KEYPAIR_PATH")
    log_info "Upgrade authority: $UPGRADE_AUTHORITY_PUBKEY"
}

show_balance() {
    log_step "Checking SOL balance"
    local balance
    balance=$(solana balance "$UPGRADE_AUTHORITY_PUBKEY" --url "$CLUSTER" 2>&1)
    log_info "Upgrade authority balance: $balance"

    local amount
    amount=$(echo "$balance" | awk '{print $1}')
    if command -v bc >/dev/null 2>&1; then
        if (( $(echo "$amount < 2" | bc -l 2>/dev/null || echo 0) )); then
            log_warn "Low balance! Program upgrades require SOL."
        fi
    fi
}

upgrade_programs() {
    log_step "Upgrading Solana programs to v${VERSION}"
    echo ""

    for entry in "${UPGRADE_PROGRAMS[@]}"; do
        IFS='|' read -r name cli_name <<< "$entry"

        # Check current version in config
        local config_key
        case "$cli_name" in
            gateway)     config_key="AxelarGateway" ;;
            gas-service) config_key="AxelarGasService" ;;
            operators)   config_key="AxelarOperators" ;;
            memo)        config_key="AxelarMemo" ;;
            its)         config_key="InterchainTokenService" ;;
        esac

        local current_version
        current_version=$(jq -r ".chains[\"${CHAIN}\"].contracts[\"${config_key}\"].version // \"unknown\"" "$CHAINS_INFO_FILE")
        local current_addr
        current_addr=$(jq -r ".chains[\"${CHAIN}\"].contracts[\"${config_key}\"].address // empty" "$CHAINS_INFO_FILE")

        log_step "Upgrade ${name}"
        log_info "Program:           ${cli_name}"
        log_info "Address:           ${current_addr:-not deployed}"
        log_info "Current version:   ${current_version}"
        log_info "New version:       ${VERSION}"
        log_info "Upgrade Authority: ${UPGRADE_AUTHORITY_PUBKEY}"

        if [[ -z "$current_addr" ]]; then
            log_warn "${name} not found in config. Skipping."
            continue
        fi

        if ! confirm "Upgrade ${name}?"; then
            log_info "Skipping ${name}"
            continue
        fi

        run_solana_cli upgrade \
            --program "$cli_name" \
            --upgrade-authority "$UPGRADE_AUTHORITY_KEYPAIR_PATH" \
            --fee-payer "$UPGRADE_AUTHORITY_KEYPAIR_PATH" \
            --version "$VERSION" \
            -y

        log_info "${name} upgraded successfully to v${VERSION}"
    done
}

print_summary() {
    log_step "Upgrade Summary"
    echo ""
    echo "    Environment: ${ENV}"
    echo "    Chain:       ${CHAIN}"
    echo "    Cluster:     ${CLUSTER}"
    echo "    Version:     ${VERSION}"
    echo ""
    echo "    Updated contracts:"
    for entry in "${UPGRADE_PROGRAMS[@]}"; do
        IFS='|' read -r name cli_name <<< "$entry"
        local config_key
        case "$cli_name" in
            gateway)     config_key="AxelarGateway" ;;
            gas-service) config_key="AxelarGasService" ;;
            operators)   config_key="AxelarOperators" ;;
            memo)        config_key="AxelarMemo" ;;
            its)         config_key="InterchainTokenService" ;;
        esac
        local new_version
        new_version=$(jq -r ".chains[\"${CHAIN}\"].contracts[\"${config_key}\"].version // \"unknown\"" "$CHAINS_INFO_FILE")
        echo "      ${name}: v${new_version}"
    done
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana Program Upgrade Script"
    log_info "Environment: ${ENV}"
    log_info "Chain:       ${CHAIN}"
    log_info "Cluster:     ${CLUSTER}"
    log_info "Version:     ${VERSION}"
    echo ""

    check_prerequisites
    resolve_upgrade_authority
    show_balance
    upgrade_programs
    print_summary

    log_step "Done!"
}

main "$@"
