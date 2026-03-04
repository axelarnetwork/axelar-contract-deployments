#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana Post-Deployment Checklist
#
# Runs post-deployment verification steps (memo init, test cross-chain message).
# Run this AFTER deploy.sh has completed deployment and initialization.
#
# Reads ENV and CHAIN from solana/.env (same file used by solana/cli).
# Fetches keypairs from 1Password on-demand and cleans them up on exit.
#
# Usage:
#   ./solana/scripts/checklist.sh
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

# =============================================================================
# Utility functions
# =============================================================================

run_solana_cli() {
    "${SOLANA_DIR}/cli" "$@"
}

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

# =============================================================================
# Checklist steps
# =============================================================================

run_memo_checklist() {
    log_step "Memo Program Checklist"

    # Initialize Memo
    log_step "Initializing Memo program"
    run_solana_cli send memo init

    local memo_keypair_path
    memo_keypair_path=$(fetch_keypair_from_op "[${ENV_DISPLAY}] Memo: Solana")
    local memo_pda
    memo_pda=$(solana-keygen pubkey "$memo_keypair_path")

    # Send test memo
    log_step "Sending test memo cross-chain"
    log_info "destination-chain:   ${CHAIN}"
    log_info "destination-address: ${memo_pda}"
    log_info "memo:                Hello"

    run_solana_cli send memo send-memo \
        --destination-chain "$CHAIN" \
        --destination-address "$memo_pda" \
        --memo "Hello"

    log_info "Test memo sent. Please verify on Axelarscan."
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana Post-Deployment Checklist"
    log_info "Environment: ${ENV}"
    log_info "Chain:       ${CHAIN}"
    echo ""

    local missing=()
    command -v solana-keygen >/dev/null 2>&1 || missing+=("solana-keygen")
    command -v cargo >/dev/null 2>&1 || missing+=("cargo")
    command -v op >/dev/null 2>&1 || missing+=("op (1Password CLI)")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi

    if confirm "Run memo program checklist?"; then
        run_memo_checklist
    fi

    log_step "Remaining steps"
    echo "    [ ] Verify cross-chain memo on Axelarscan"
    echo "    [ ] Run ./solana/scripts/its-checklist.sh for ITS verification"
    echo ""

    log_step "Done!"
}

main "$@"
