#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana ITS Post-Deployment Checklist
#
# Interactive guided checklist for ITS token operations verification.
# Run this AFTER deploy.sh and setup-its.sh have completed.
#
# Sections:
#   A. Deploy Native Interchain Token (Solana -> EVM)
#   B. Deploy Canonical Token
#   C. Interchain Transfer verification
#   D. EVM -> Solana direction (prints commands for user)
#
# Each section is optional — prompts before running.
#
# Reads ENV and CHAIN from solana/.env (same file used by solana/cli).
#
# Usage:
#   ./solana/scripts/its-checklist.sh
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

prompt_value() {
    local varname="$1"
    local prompt="$2"
    local default="${3:-}"
    local value

    if [[ -n "$default" ]]; then
        read -r -p "    ${prompt} [${default}]: " value
        value="${value:-$default}"
    else
        read -r -p "    ${prompt}: " value
    fi

    if [[ -z "$value" ]]; then
        log_error "${varname} is required."
        return 1
    fi

    eval "${varname}=\"${value}\""
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
    echo "Interactive ITS token operations verification checklist."
    echo "Reads ENV and CHAIN from solana/.env."
    echo ""
    echo "Sections:"
    echo "  A. Deploy Native Interchain Token (Solana -> EVM)"
    echo "  B. Deploy Canonical Token"
    echo "  C. Interchain Transfer verification"
    echo "  D. EVM -> Solana direction (prints commands)"
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

# =============================================================================
# Checklist sections
# =============================================================================

section_deploy_native_interchain_token() {
    log_step "A. Deploy Native Interchain Token (Solana -> EVM)"
    echo ""
    echo "    This deploys a new interchain token on Solana and a remote"
    echo "    counterpart on an EVM destination chain."
    echo ""

    if ! confirm "Run this section?"; then
        log_info "Skipping section A."
        return
    fi

    # Deploy interchain token
    log_step "A.1 Deploy Interchain Token"
    local SALT NAME SYMBOL DECIMALS INITIAL_SUPPLY
    prompt_value SALT "Salt (hex string)"
    prompt_value NAME "Token name"
    prompt_value SYMBOL "Token symbol"
    prompt_value DECIMALS "Decimals" "9"
    prompt_value INITIAL_SUPPLY "Initial supply" "1000"

    run_solana_cli send its deploy-interchain-token \
        --salt "$SALT" \
        --name "$NAME" \
        --symbol "$SYMBOL" \
        --decimals "$DECIMALS" \
        --initial-supply "$INITIAL_SUPPLY"

    log_info "Interchain token deployed"

    # Deploy remote interchain token
    log_step "A.2 Deploy Remote Interchain Token"
    local DEST_CHAIN GAS_VALUE
    prompt_value DEST_CHAIN "Destination chain"
    prompt_value GAS_VALUE "Gas value (lamports)" "500000"

    run_solana_cli send its deploy-remote-interchain-token \
        --salt "$SALT" \
        --destination-chain "$DEST_CHAIN" \
        --gas-value "$GAS_VALUE"

    log_info "Remote interchain token deployed"
    echo ""
    echo "    Verify the deployment on Axelarscan before continuing."
    confirm "Verified on Axelarscan?" || true

    # Interchain transfer
    if confirm "Test interchain transfer for this token?"; then
        log_step "A.3 Interchain Transfer"
        local SOURCE_ACCOUNT TOKEN_ID DEST_ADDRESS AMOUNT
        prompt_value SOURCE_ACCOUNT "Source token account (SOLANA_MINT_TOKEN_MANAGER_ADDRESS)"
        prompt_value TOKEN_ID "Token ID"
        prompt_value DEST_ADDRESS "Destination address (EVM)"
        prompt_value AMOUNT "Amount"
        prompt_value GAS_VALUE "Gas value (lamports)" "500000"

        run_solana_cli send its interchain-transfer \
            --source-account "$SOURCE_ACCOUNT" \
            --token-id "$TOKEN_ID" \
            --destination-chain "$DEST_CHAIN" \
            --destination-address "$DEST_ADDRESS" \
            --amount "$AMOUNT" \
            --gas-value "$GAS_VALUE"

        log_info "Interchain transfer sent"
        echo ""
        echo "    Verify the transfer on Axelarscan."
        confirm "Verified?" || true
    fi
}

section_deploy_canonical_token() {
    log_step "B. Deploy Canonical Token"
    echo ""
    echo "    This registers an existing Solana token as a canonical interchain"
    echo "    token and deploys a remote representation."
    echo ""

    if ! confirm "Run this section?"; then
        log_info "Skipping section B."
        return
    fi

    # Register canonical token
    log_step "B.1 Register Canonical Interchain Token"
    local MINT
    prompt_value MINT "Mint address (existing Solana token)"

    run_solana_cli send its register-canonical-interchain-token \
        --mint "$MINT"

    log_info "Canonical interchain token registered"

    # Deploy remote canonical token
    if confirm "Deploy remote canonical token?"; then
        log_step "B.2 Deploy Remote Canonical Interchain Token"
        local DEST_CHAIN GAS_VALUE
        prompt_value DEST_CHAIN "Destination chain"
        prompt_value GAS_VALUE "Gas value (lamports)" "500000"

        run_solana_cli send its deploy-remote-canonical-interchain-token \
            --mint "$MINT" \
            --destination-chain "$DEST_CHAIN" \
            --gas-value "$GAS_VALUE"

        log_info "Remote canonical token deployed"
        echo ""
        echo "    Verify the deployment on Axelarscan."
        confirm "Verified?" || true
    fi

    # Interchain transfer for canonical token
    if confirm "Test interchain transfer for canonical token?"; then
        log_step "B.3 Interchain Transfer (Canonical)"
        local SOURCE_ACCOUNT TOKEN_ID DEST_CHAIN DEST_ADDRESS AMOUNT GAS_VALUE
        prompt_value SOURCE_ACCOUNT "Source token account"
        prompt_value TOKEN_ID "Token ID"
        prompt_value DEST_CHAIN "Destination chain"
        prompt_value DEST_ADDRESS "Destination address"
        prompt_value AMOUNT "Amount"
        prompt_value GAS_VALUE "Gas value (lamports)" "500000"

        run_solana_cli send its interchain-transfer \
            --source-account "$SOURCE_ACCOUNT" \
            --token-id "$TOKEN_ID" \
            --destination-chain "$DEST_CHAIN" \
            --destination-address "$DEST_ADDRESS" \
            --amount "$AMOUNT" \
            --gas-value "$GAS_VALUE"

        log_info "Interchain transfer sent"
        echo ""
        echo "    Verify the transfer on Axelarscan."
        confirm "Verified?" || true
    fi
}

section_evm_to_solana() {
    log_step "D. EVM -> Solana Direction"
    echo ""
    echo "    The following steps require an EVM PRIVATE_KEY."
    echo "    Commands are printed for you to run manually."
    echo ""

    if ! confirm "Show EVM -> Solana commands?"; then
        log_info "Skipping section D."
        return
    fi

    echo ""
    echo "    Tips:"
    echo "      - Get mint address from token ID:"
    echo "        solana/cli query its token-manager <TOKEN_ID>"
    echo ""
    echo "      - When transferring to Solana, use a token account (not wallet address)."
    echo "        Create one with: spl-token create-account --owner <WALLET> <MINT>"
    echo ""

    log_step "D.1 Deploy Native Interchain Token (EVM -> Solana)"
    echo ""
    echo "    # Set EVM PRIVATE_KEY in .env, then:"
    echo ""
    echo "    ts-node evm/interchainTokenFactory.js deploy-interchain-token \\"
    echo "        --name <TOKEN_NAME> \\"
    echo "        --symbol <TOKEN_SYMBOL> \\"
    echo "        --decimals <TOKEN_DECIMALS> \\"
    echo "        --initialSupply <TOKEN_INITIAL_SUPPLY> \\"
    echo "        --minter <TOKEN_MINTER> \\"
    echo "        --chainNames <SOURCE_CHAIN> \\"
    echo "        --salt <SALT>"
    echo ""
    echo "    ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token ${CHAIN} \\"
    echo "        --chainNames <SOURCE_CHAIN> \\"
    echo "        --salt <SALT>"
    echo ""

    log_step "D.2 Interchain Transfer (EVM -> Solana)"
    echo ""
    echo "    ts-node evm/its.js interchain-transfer \\"
    echo "        --destinationChain ${CHAIN} \\"
    echo "        --tokenId <TOKEN_ID> \\"
    echo "        --destinationAddress <SOLANA_TOKEN_ACCOUNT> \\"
    echo "        --amount <AMOUNT> \\"
    echo "        --chainNames <SOURCE_CHAIN>"
    echo ""

    log_step "D.3 Deploy Remote Canonical Token (EVM -> Solana)"
    echo ""
    echo "    ts-node evm/interchainTokenFactory.js register-canonical-interchain-token \\"
    echo "        --tokenAddress <EVM_TOKEN_ADDRESS> \\"
    echo "        --chainNames <SOURCE_CHAIN>"
    echo ""
    echo "    ts-node evm/interchainTokenFactory.js deploy-remote-canonical-interchain-token \\"
    echo "        <EVM_TOKEN_ADDRESS> ${CHAIN} \\"
    echo "        --chainNames <SOURCE_CHAIN>"
    echo ""

    confirm "Acknowledged?" || true
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana ITS Post-Deployment Checklist"
    log_info "Environment: ${ENV}"
    log_info "Chain:       ${CHAIN}"
    echo ""

    local missing=()
    command -v cargo >/dev/null 2>&1 || missing+=("cargo")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi

    section_deploy_native_interchain_token
    section_deploy_canonical_token
    section_evm_to_solana

    log_step "Checklist complete"
    echo ""
    echo "    [ ] Verify all cross-chain transactions on Axelarscan"
    echo "    [ ] Confirm token balances on both source and destination chains"
    echo ""

    log_step "Done!"
}

main "$@"
