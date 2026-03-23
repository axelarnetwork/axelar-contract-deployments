#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana Key Setup Script
#
# Generates and stores keypairs needed before deploying Solana programs.
# Run this BEFORE deploy.sh. After running, commit the updated program IDs
# in solana-axelar, publish a release, and use the resulting version with
# deploy.sh.
#
# Reads ENV from solana/.env (same file used by solana/cli).
#
# Usage:
#   ./solana/scripts/setup-keys.sh --generate-program-ids
#   ./solana/scripts/setup-keys.sh --generate-keypairs
#   ./solana/scripts/setup-keys.sh --generate-program-ids --generate-keypairs
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

# =============================================================================
# Environment-dependent configuration
# =============================================================================

get_op_vault() {
    case "$ENV" in
        devnet-amplifier) echo "Devnet - Axelar Externally Owned Accounts" ;;
        stagenet)         echo "Stagenet - Axelar Externally Owned Accounts" ;;
        testnet)          echo "Testnet - Axelar Externally Owned Accounts" ;;
        mainnet)          echo "Mainnet - Axelar Externally Owned Accounts" ;;
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

# =============================================================================
# Flag parsing
# =============================================================================

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Generates and stores keypairs for Solana program deployment."
    echo "Reads ENV from solana/.env."
    echo ""
    echo "Options (at least one required):"
    echo "  --generate-program-ids     Run cargo xtask update-ids and import program keypairs to 1Password"
    echo "  --generate-keypairs        Generate new upgrade-authority (upa) and operator (gop) keypairs"
    echo "  --solana-axelar-dir <dir>  Path to solana-axelar repo (default: ../axelar-amplifier-solana)"
    echo "  -h, --help                 Show this help"
    echo ""
    echo "After running with --generate-program-ids:"
    echo "  1. Commit the updated program IDs in solana-axelar"
    echo "  2. Create a release (publish to crates.io)"
    echo "  3. Use the released version with ./solana/scripts/deploy.sh --version <ver>"
}

FLAG_GENERATE_PROGRAM_IDS=false
FLAG_GENERATE_KEYPAIRS=false
SOLANA_AXELAR_DIR_ARG=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --generate-program-ids)
            FLAG_GENERATE_PROGRAM_IDS=true
            shift
            ;;
        --generate-keypairs)
            FLAG_GENERATE_KEYPAIRS=true
            shift
            ;;
        --solana-axelar-dir)
            SOLANA_AXELAR_DIR_ARG="$2"
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

if [[ "$FLAG_GENERATE_PROGRAM_IDS" == false ]] && [[ "$FLAG_GENERATE_KEYPAIRS" == false ]]; then
    log_error "At least one of --generate-program-ids or --generate-keypairs is required"
    usage
    exit 1
fi

# Resolve solana-axelar directory
if [[ -n "$SOLANA_AXELAR_DIR_ARG" ]]; then
    SOLANA_AXELAR_DIR="$(cd "$SOLANA_AXELAR_DIR_ARG" && pwd)"
else
    SOLANA_AXELAR_DIR="$(cd "${DEPLOYMENTS_DIR}/../solana-axelar" 2>/dev/null && pwd)" || {
        log_error "solana-axelar directory not found at ${DEPLOYMENTS_DIR}/../solana-axelar"
        log_info "Use --solana-axelar-dir to specify the path"
        exit 1
    }
fi

# =============================================================================
# Utility functions
# =============================================================================

find_keypair_by_prefix() {
    local prefix="$1"
    local dir="$2"
    find "$dir" -maxdepth 1 -name "${prefix}*.json" -type f 2>/dev/null | head -1
}

pubkey_from_path() {
    basename "$1" .json
}

# =============================================================================
# Step functions
# =============================================================================

check_prerequisites() {
    log_step "Checking prerequisites"

    local missing=()
    command -v solana-keygen >/dev/null 2>&1 || missing+=("solana-keygen")
    command -v cargo >/dev/null 2>&1 || missing+=("cargo")
    command -v op >/dev/null 2>&1 || missing+=("op (1Password CLI)")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi
    log_info "All tools available"

    if [[ "$FLAG_GENERATE_PROGRAM_IDS" == true ]]; then
        if [[ ! -d "$SOLANA_AXELAR_DIR" ]]; then
            log_error "solana-axelar directory not found: $SOLANA_AXELAR_DIR"
            exit 1
        fi
        if [[ ! -f "${SOLANA_AXELAR_DIR}/Anchor.toml" ]]; then
            log_error "Anchor.toml not found in $SOLANA_AXELAR_DIR"
            exit 1
        fi
        log_info "solana-axelar directory verified: $SOLANA_AXELAR_DIR"
    fi
}

update_env_var() {
    local key="$1"
    local value="$2"
    local env_file="${SOLANA_DIR}/.env"

    if grep -q "^${key}=" "$env_file" 2>/dev/null; then
        # Update existing line
        sed -i.bak "s|^${key}=.*|${key}=${value}|" "$env_file"
        rm -f "${env_file}.bak"
    else
        # Append new line
        echo "${key}=${value}" >> "$env_file"
    fi
}

generate_keypairs() {
    log_step "Generating upgrade-authority and operator keypairs"
    log_warn "This uses solana-keygen grind which may take a few minutes per keypair."

    if ! confirm "Generate new upgrade-authority (upa) and operator (gop) keypairs?"; then
        log_info "Skipping keypair generation"
        return
    fi

    # Generate into solana/deployments/ directory
    mkdir -p "${SOLANA_DIR}/deployments"

    # Generate upgrade authority
    log_info "Generating upgrade authority keypair (prefix: upa)..."
    pushd "${SOLANA_DIR}/deployments" > /dev/null
    solana-keygen grind --starts-with upa:1
    local upa_file
    upa_file=$(ls -t upa*.json 2>/dev/null | head -1)
    popd > /dev/null

    if [[ -z "$upa_file" ]]; then
        log_error "Failed to find generated upgrade authority keypair"
        exit 1
    fi
    local upa_path="${SOLANA_DIR}/deployments/${upa_file}"
    local upa_pubkey
    upa_pubkey=$(pubkey_from_path "$upa_path")
    log_info "Generated upgrade authority: $upa_pubkey"
    log_info "  Path: $upa_path"

    # Generate operator
    log_info "Generating operator keypair (prefix: gop)..."
    pushd "${SOLANA_DIR}/deployments" > /dev/null
    solana-keygen grind --starts-with gop:1
    local gop_file
    gop_file=$(ls -t gop*.json 2>/dev/null | head -1)
    popd > /dev/null

    if [[ -z "$gop_file" ]]; then
        log_error "Failed to find generated operator keypair"
        exit 1
    fi
    local gop_path="${SOLANA_DIR}/deployments/${gop_file}"
    local gop_pubkey
    gop_pubkey=$(pubkey_from_path "$gop_path")
    log_info "Generated operator: $gop_pubkey"
    log_info "  Path: $gop_path"

    # Update solana/.env with paths
    log_info "Updating solana/.env with keypair paths..."
    update_env_var "UPGRADE_AUTHORITY_KEYPAIR_PATH" "./deployments/${upa_file}"
    update_env_var "OPERATOR_KEYPAIR_PATH" "./deployments/${gop_file}"
    log_info "Updated UPGRADE_AUTHORITY_KEYPAIR_PATH=./deployments/${upa_file}"
    log_info "Updated OPERATOR_KEYPAIR_PATH=./deployments/${gop_file}"

    # Import to 1Password
    log_step "Importing authority/operator keypairs to 1Password"
    log_info "Vault: $OP_VAULT"

    local upa_item_id
    upa_item_id=$(op document create "$upa_path" \
        --title "[${ENV_DISPLAY}] Upgrade Authority: Solana" \
        --vault "$OP_VAULT" \
        --tags "solana,upgrade-authority,${ENV}" \
        --format json | jq -r '.uuid')
    op item edit "$upa_item_id" --vault "$OP_VAULT" \
        "notesPlain=Pubkey: ${upa_pubkey}" > /dev/null
    log_info "Imported: [${ENV_DISPLAY}] Upgrade Authority: Solana (${upa_pubkey})"

    local gop_item_id
    gop_item_id=$(op document create "$gop_path" \
        --title "[${ENV_DISPLAY}] Operator: Solana" \
        --vault "$OP_VAULT" \
        --tags "solana,operator,${ENV}" \
        --format json | jq -r '.uuid')
    op item edit "$gop_item_id" --vault "$OP_VAULT" \
        "notesPlain=Pubkey: ${gop_pubkey}" > /dev/null
    log_info "Imported: [${ENV_DISPLAY}] Operator: Solana (${gop_pubkey})"
}

generate_program_ids() {
    log_step "Regenerating program IDs via: cargo xtask update-ids --network $ENV"
    log_warn "This runs solana-keygen grind for 6 programs. May take several minutes."

    if ! confirm "Continue?"; then
        log_info "Skipping"
        return
    fi

    pushd "$SOLANA_AXELAR_DIR" > /dev/null
    cargo xtask update-ids --network "$ENV"
    popd > /dev/null

    # Import generated program keypairs to 1Password
    log_step "Importing program keypairs to 1Password"
    log_info "Vault: $OP_VAULT"

    local names=("Gateway" "Gas Service" "Governance" "ITS" "Memo" "Operators")
    local prefixes=("gtw" "gas" "gov" "its" "mem" "opr")

    for i in "${!prefixes[@]}"; do
        local prefix="${prefixes[$i]}"
        local name="${names[$i]}"
        local keypair_file
        keypair_file=$(find_keypair_by_prefix "$prefix" "$SOLANA_AXELAR_DIR")

        if [[ -n "$keypair_file" ]]; then
            local pubkey
            pubkey=$(pubkey_from_path "$keypair_file")
            log_info "Importing ${name} (${pubkey})..."
            op document create "$keypair_file" \
                --title "[${ENV_DISPLAY}] ${name}: Solana" \
                --vault "$OP_VAULT"
        else
            log_warn "No keypair file found for ${name} (prefix: ${prefix})"
        fi
    done

    log_info "All program keypairs imported to 1Password"

    echo ""
    log_step "Next steps"
    echo "    1. Review the updated declare_id! macros in solana-axelar"
    echo "    2. Update Anchor.toml [programs.${ENV}] section (xtask printed the block above)"
    echo "    3. Commit and push the changes"
    echo "    4. Create a release and publish to crates.io"
    echo "    5. Use the released version: ./solana/scripts/deploy.sh --version <VERSION>"
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana Key Setup"
    log_info "Environment: ${ENV}"
    log_info "Vault:       ${OP_VAULT}"
    if [[ "$FLAG_GENERATE_PROGRAM_IDS" == true ]]; then
        log_info "solana-axelar: ${SOLANA_AXELAR_DIR}"
    fi
    echo ""

    check_prerequisites

    if [[ "$FLAG_GENERATE_KEYPAIRS" == true ]]; then
        generate_keypairs
    fi

    if [[ "$FLAG_GENERATE_PROGRAM_IDS" == true ]]; then
        generate_program_ids
    fi

    log_step "Done!"
}

main "$@"
