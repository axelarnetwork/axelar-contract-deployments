#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana Post-Deployment Checklist
#
# Runs post-deployment verification steps (test cross-chain memo message).
# Run this AFTER deploy.sh has completed deployment and initialization.
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

CHAINS_INFO_FILE="${DEPLOYMENTS_DIR}/axelar-chains-config/info/${ENV}.json"

# =============================================================================
# Utility functions
# =============================================================================

run_solana_cli() {
    "${SOLANA_DIR}/cli" "$@"
}

# =============================================================================
# Checklist steps
# =============================================================================

run_memo_checklist() {
    log_step "Memo Program Checklist"

    local memo_pda
    memo_pda=$(jq -r ".chains[\"${CHAIN}\"].contracts.AxelarMemo.address // empty" "$CHAINS_INFO_FILE")
    if [[ -z "$memo_pda" ]]; then
        log_error "AxelarMemo address not found for '${CHAIN}' in config."
        log_info "Run deploy.sh first to deploy the Memo program."
        return
    fi

    # Send test memo
    log_step "Sending test memo cross-chain"
    log_info "destination-chain:   ${CHAIN}"
    log_info "destination-address: ${memo_pda}"
    log_info "memo:                Hello"

    local memo_output
    memo_output=$(run_solana_cli send memo send-memo \
        --destination-chain "$CHAIN" \
        --destination-address "$memo_pda" \
        --memo "Hello" 2>&1) || true
    echo "$memo_output"

    local tx_sig
    tx_sig=$(echo "$memo_output" | sed -n 's/.*Transaction Signature (ID): \([^ ]*\).*/\1/p' | head -1)

    if [[ -z "$tx_sig" ]]; then
        log_warn "Could not parse transaction signature from output."
        read -r -p "    Enter the transaction signature manually: " tx_sig
        if [[ -z "$tx_sig" ]]; then
            log_error "Transaction signature is required to pay gas."
            return
        fi
    fi

    log_info "Test memo sent. TX: ${tx_sig}"

    # Pay gas for the memo message
    log_step "Paying gas for memo message"

    local message_id="${tx_sig}-1"
    log_info "message-id:     ${message_id}"

    local gas_amount
    read -r -p "    Gas amount in lamports [500000]: " gas_amount
    gas_amount="${gas_amount:-500000}"

    local refund_address
    refund_address=$(solana address 2>/dev/null || echo "")
    read -r -p "    Refund address [${refund_address}]: " input_refund
    refund_address="${input_refund:-$refund_address}"

    log_info "amount:          ${gas_amount}"
    log_info "refund-address:  ${refund_address}"

    run_solana_cli send gas-service add-gas \
        --message-id "$message_id" \
        --amount "$gas_amount" \
        --refund-address "$refund_address"

    local axelarscan_subdomain
    case "$ENV" in
        devnet-amplifier) axelarscan_subdomain="devnet-amplifier" ;;
        stagenet)         axelarscan_subdomain="stagenet" ;;
        testnet)          axelarscan_subdomain="testnet" ;;
        mainnet)          axelarscan_subdomain="" ;;
    esac

    local axelarscan_url
    if [[ -n "$axelarscan_subdomain" ]]; then
        axelarscan_url="https://${axelarscan_subdomain}.axelarscan.io/gmp/${message_id}"
    else
        axelarscan_url="https://axelarscan.io/gmp/${message_id}"
    fi

    log_info "Gas added. Verify on Axelarscan:"
    log_info "$axelarscan_url"
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana Post-Deployment Checklist"
    log_info "Environment: ${ENV}"
    log_info "Chain:       ${CHAIN}"
    echo ""

    if [[ ! -f "$CHAINS_INFO_FILE" ]]; then
        log_error "Chains info file not found: $CHAINS_INFO_FILE"
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
