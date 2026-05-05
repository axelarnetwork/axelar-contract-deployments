#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana ITS Hub Registration & Trusted Chain Setup
#
# Registers the Solana ITS chain on the ITS Hub and configures trusted chains.
# Run this AFTER deploy.sh has deployed and initialized the ITS program.
#
# Steps:
#   1. Add ITS Hub config entry for the chain
#   2. Register chain on ITS Hub (CosmWasm)
#   3. Set trusted chains on Solana ITS
#   4. Print EVM trusted chain command (requires EVM PRIVATE_KEY)
#
# Reads ENV and CHAIN from solana/.env (same file used by solana/cli).
#
# Usage:
#   ./solana/scripts/setup-its.sh
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
        devnet-amplifier|testnet) echo "devnet" ;;
        stagenet)                 echo "testnet" ;;
        mainnet)                  echo "mainnet-beta" ;;
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
OP_VAULT=$(get_op_vault)
ENV_DISPLAY=$(get_env_display)
CHAINS_INFO_FILE="${DEPLOYMENTS_DIR}/axelar-chains-config/info/${ENV}.json"

# Export so child processes (cli wrapper → cargo run → CLI binary) pick them up via clap env
export CLUSTER
export CHAIN

# Governance flag: empty on devnet (direct execution), --governance elsewhere
if [[ "$ENV" == "devnet-amplifier" ]]; then
    GOVERNANCE_FLAG=""
else
    GOVERNANCE_FLAG="--governance"
fi

# Track temporary files for cleanup
TEMP_FILES=()

cleanup() {
    if [[ ${#TEMP_FILES[@]} -gt 0 ]]; then
        for f in "${TEMP_FILES[@]}"; do
            if [[ -f "$f" ]]; then
                rm -f "$f"
            fi
        done
    fi
}
trap cleanup EXIT

# =============================================================================
# Flag parsing
# =============================================================================

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Registers Solana ITS on the ITS Hub and configures trusted chains."
    echo "Reads ENV and CHAIN from solana/.env."
    echo ""
    echo "Options:"
    echo "  -h, --help    Show this help"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
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
# Utility functions
# =============================================================================

run_solana_cli() {
    "${SOLANA_DIR}/cli" "$@"
}

run_ts_node() {
    log_info "Running: ts-node $*"
    (cd "$DEPLOYMENTS_DIR" && ts-node "$@")
}

jq_config() {
    jq -r "$1" "$CHAINS_INFO_FILE"
}

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

# =============================================================================
# Step functions
# =============================================================================

check_prerequisites() {
    log_step "Checking prerequisites"

    local missing=()
    command -v jq >/dev/null 2>&1 || missing+=("jq")
    command -v ts-node >/dev/null 2>&1 || missing+=("ts-node")
    command -v node >/dev/null 2>&1 || missing+=("node")

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

    # Validate ITS is deployed on Solana
    local its_addr
    its_addr=$(jq_config ".chains[\"${CHAIN}\"].contracts.InterchainTokenService.address // empty")
    if [[ -z "$its_addr" ]]; then
        log_error "InterchainTokenService not found for chain '${CHAIN}' in config."
        log_info "Run deploy.sh first to deploy and initialize ITS."
        exit 1
    fi
    log_info "Solana ITS: $its_addr"

    # Validate ITS Hub exists
    local its_hub_addr
    its_hub_addr=$(jq_config '.axelar.contracts.InterchainTokenService.address // empty')
    if [[ -z "$its_hub_addr" ]]; then
        log_error "ITS Hub (axelar.contracts.InterchainTokenService.address) not found in config."
        exit 1
    fi
    log_info "ITS Hub: $its_hub_addr"

    log_info "All prerequisites verified"
}

step_add_its_hub_config() {
    log_step "Step 1: Add ITS Hub config entry for ${CHAIN}"

    local existing
    existing=$(jq_config ".axelar.contracts.InterchainTokenService[\"${CHAIN}\"] // empty")

    if [[ -n "$existing" ]]; then
        log_info "ITS Hub config for '${CHAIN}' already exists:"
        jq ".axelar.contracts.InterchainTokenService[\"${CHAIN}\"]" "$CHAINS_INFO_FILE" | sed 's/^/    /'
        if ! confirm "Overwrite existing config?"; then
            log_info "Skipping."
            return
        fi
    fi

    # Read ItsSolanaTranslator address from config
    local translator_address
    translator_address=$(jq_config ".axelar.contracts.ItsSolanaTranslator.address // empty")
    if [[ -z "$translator_address" ]]; then
        log_error "ItsSolanaTranslator address not found in config."
        log_info "Ensure the ItsSolanaTranslator contract is deployed (via deploy-axelar.sh)."
        exit 1
    fi
    log_info "ItsSolanaTranslator: $translator_address"

    local tmp_file="${CHAINS_INFO_FILE}.tmp"
    jq --arg chain "$CHAIN" \
       --arg translator "$translator_address" \
    '.axelar.contracts.InterchainTokenService[$chain] = {
        "maxUintBits": 64,
        "maxDecimalsWhenTruncating": 6,
        "msgTranslator": $translator
    }' "$CHAINS_INFO_FILE" > "$tmp_file" && mv "$tmp_file" "$CHAINS_INFO_FILE"

    log_info "Added ITS Hub config for '${CHAIN}' with msgTranslator=$translator_address"
}

step_register_chain_on_its_hub() {
    log_step "Step 2: Register ${CHAIN} on ITS Hub"

    local existing_chain
    existing_chain=$(jq_config ".axelar.contracts.InterchainTokenService[\"${CHAIN}\"].address // empty")

    local action="its-hub-register-chains"
    if [[ -n "$existing_chain" ]]; then
        log_warn "${CHAIN} may already be registered on ITS Hub."
        echo "    Use 'register' for new chains or 'update' to modify existing registration."
        echo ""
        echo "    1) Register (its-hub-register-chains) — for new chains"
        echo "    2) Update (its-hub-update-chains) — to update existing registration"
        echo ""
        read -r -p "    Choose [1/2]: " choice
        case "$choice" in
            2) action="its-hub-update-chains" ;;
            *) action="its-hub-register-chains" ;;
        esac
    fi

    if ! confirm "Run ${action} for ${CHAIN}?"; then
        log_info "Skipping."
        return
    fi

    log_info "Action: $action"
    log_info "Chain:  $CHAIN"

    if [[ -n "$GOVERNANCE_FLAG" ]]; then
        log_info "Governance proposal will be created."
    fi

    # shellcheck disable=SC2086
    run_ts_node cosmwasm/contract.ts "$action" "$CHAIN" \
        -m "$MNEMONIC" \
        $GOVERNANCE_FLAG

    if [[ -n "$GOVERNANCE_FLAG" ]]; then
        echo ""
        log_step "Governance Proposal: ITS Hub chain registration"
        echo "    The governance proposal has been submitted."
        echo "    Please wait for it to pass before continuing."
        echo ""

        if ! confirm "Has the governance proposal passed?"; then
            log_warn "Script paused. Re-run to resume."
            exit 0
        fi
    fi

    log_info "Chain registered on ITS Hub"
}

step_set_trusted_chains_solana() {
    log_step "Step 3: Set trusted chains on Solana ITS"

    log_info "This adds all registered chains as trusted on the Solana ITS program."
    echo ""

    if ! confirm "Set all trusted chains on Solana ITS?"; then
        log_info "Skipping."
        return
    fi

    run_solana_cli send its set-trusted-chain all

    log_info "Trusted chains set on Solana ITS"
}

step_set_trusted_chains_evm() {
    log_step "Step 4: Set Solana as trusted chain on EVM ITS"

    echo ""
    echo "    This step requires an EVM PRIVATE_KEY and must be run separately."
    echo ""
    echo "    Run the following command:"
    echo ""
    if [[ -n "$GOVERNANCE_FLAG" ]]; then
        echo "      ts-node evm/its.js set-trusted-chains ${CHAIN} hub -n all --governance"
    else
        echo "      ts-node evm/its.js set-trusted-chains ${CHAIN} hub -n all"
    fi
    echo ""
    echo "    Make sure PRIVATE_KEY is set in your .env for the EVM deployer."
    echo ""

    confirm "Acknowledged?" || true
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana ITS Hub Registration & Trusted Chain Setup"
    log_info "Environment: ${ENV}"
    log_info "Chain:       ${CHAIN}"
    echo ""

    check_prerequisites
    resolve_mnemonic

    step_add_its_hub_config
    step_register_chain_on_its_hub
    step_set_trusted_chains_solana
    step_set_trusted_chains_evm

    log_step "Summary"
    echo ""
    echo "    ITS Hub registration complete for '${CHAIN}'."
    echo ""
    echo "    Next: run ./solana/scripts/checklist.sh for memo verification"
    echo "    Then: run ./solana/scripts/its-checklist.sh for ITS verification"
    echo ""

    log_step "Done!"
}

main "$@"
