#!/bin/bash
set -euo pipefail

# =============================================================================
# Solana Program Verification Script (Mainnet only)
#
# Reproduces deployed Solana program binaries from source and uploads the
# verification PDA on-chain, signed by the program's upgrade authority.
# Then queues an OtterSec remote build, which is what makes block explorers
# (Solscan, SolanaFM, etc.) display the "verified" badge.
#
# Fetches the upgrade-authority keypair from 1Password on-demand and cleans
# it up on exit.
#
# Prerequisites:
#   - Programs already deployed to mainnet (see deploy.sh)
#   - 1Password CLI (op) authenticated
#   - Docker daemon running (image is amd64; on Apple Silicon Rosetta is used)
#
# Usage:
#   ./solana/scripts/verify.sh --commit-hash <SHA>
#   ./solana/scripts/verify.sh --commit-hash <SHA> --only gateway
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOLANA_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEPLOYMENTS_DIR="$(cd "${SOLANA_DIR}/.." && pwd)"

log_step()  { echo -e "\n\033[1;34m==> $1\033[0m"; }
log_info()  { echo "    $1"; }
log_warn()  { echo -e "    \033[1;33mWARNING: $1\033[0m"; }
log_error() { echo -e "\033[1;31mERROR: $1\033[0m"; }

CHAIN="solana"
FEATURE="mainnet"
OP_VAULT="Mainnet - Axelar Externally Owned Accounts"
CHAINS_INFO_FILE="${DEPLOYMENTS_DIR}/axelar-chains-config/info/mainnet.json"

# Default RPC URL. Override with --rpc-url or SOLANA_MAINNET_RPC_URL env var.
# Public mainnet-beta rate-limits aggressively; a paid RPC is recommended.
DEFAULT_RPC_URL="${SOLANA_MAINNET_RPC_URL:-mainnet-beta}"

# Pinned base image. Must contain platform-tools matching what built the
# deployed binary. See docs/verifiable-builds.md in axelar-amplifier-solana.
BASE_IMAGE="solanafoundation/solana-verifiable-build:3.1.12"

REPO_URL="https://github.com/axelarnetwork/axelar-amplifier-solana"

# Programs: "display_name|library_name|chains_config_key"
VERIFY_PROGRAMS=(
    "Gateway|solana_axelar_gateway|AxelarGateway"
    "Gas Service|solana_axelar_gas_service|AxelarGasService"
    "Operators|solana_axelar_operators|AxelarOperators"
    "Memo|solana_axelar_memo|AxelarMemo"
    "ITS|solana_axelar_its|InterchainTokenService"
)

TEMP_KEYPAIR_FILES=()

cleanup() {
    if [[ ${#TEMP_KEYPAIR_FILES[@]} -gt 0 ]]; then
        log_info "Cleaning up temporary keypair files..."
        for f in "${TEMP_KEYPAIR_FILES[@]}"; do
            [[ -f "$f" ]] && rm -f "$f"
        done
    fi
}
trap cleanup EXIT

# =============================================================================
# Flag parsing
# =============================================================================

usage() {
    echo "Usage: $0 --commit-hash <SHA> [--only <name>] [--rpc-url <url>]"
    echo ""
    echo "Required:"
    echo "  --commit-hash <sha>   Full git SHA in axelarnetwork/axelar-amplifier-solana"
    echo "                        corresponding to the deployed binaries"
    echo ""
    echo "Optional:"
    echo "  --only <name>         Verify a single program by short name"
    echo "                        (gateway, gas-service, operators, memo, its)"
    echo "  --rpc-url <url>       Solana RPC URL or moniker (default: \$SOLANA_MAINNET_RPC_URL"
    echo "                        or 'mainnet-beta'). Public mainnet-beta rate-limits"
    echo "                        aggressively; a paid RPC is recommended."
    echo "  --skip-build          Skip the local docker build (only upload PDA)"
    echo "  -h, --help            Show this help"
}

COMMIT_HASH=""
ONLY=""
SKIP_BUILD_FLAG=""
RPC_URL="$DEFAULT_RPC_URL"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --commit-hash)  COMMIT_HASH="$2"; shift 2 ;;
        --only)         ONLY="$2"; shift 2 ;;
        --rpc-url)      RPC_URL="$2"; shift 2 ;;
        --skip-build)   SKIP_BUILD_FLAG="--skip-build"; shift ;;
        -h|--help)      usage; exit 0 ;;
        *)              log_error "Unknown flag: $1"; usage; exit 1 ;;
    esac
done

if [[ -z "$COMMIT_HASH" ]]; then
    log_error "--commit-hash is required"
    usage
    exit 1
fi

if [[ ! "$COMMIT_HASH" =~ ^[a-fA-F0-9]{40}$ ]]; then
    log_error "Invalid commit hash: $COMMIT_HASH (must be a 40-character SHA)"
    exit 1
fi

# =============================================================================
# Step functions
# =============================================================================

check_prerequisites() {
    log_step "Checking prerequisites"

    local missing=()
    command -v solana >/dev/null 2>&1 || missing+=("solana")
    command -v solana-keygen >/dev/null 2>&1 || missing+=("solana-keygen")
    command -v solana-verify >/dev/null 2>&1 || missing+=("solana-verify (cargo install solana-verify)")
    command -v docker >/dev/null 2>&1 || missing+=("docker")
    command -v jq >/dev/null 2>&1 || missing+=("jq")
    command -v op >/dev/null 2>&1 || missing+=("op (1Password CLI)")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi

    if ! docker info >/dev/null 2>&1; then
        log_error "Docker daemon is not running"
        exit 1
    fi

    if [[ ! -f "$CHAINS_INFO_FILE" ]]; then
        log_error "Chains info file not found: $CHAINS_INFO_FILE"
        exit 1
    fi

    log_info "All tools available"
}

# Fetch a document from 1Password by title; track for cleanup.
fetch_keypair_from_op() {
    local title="$1"
    mkdir -p "${SOLANA_DIR}/deployments"
    local sanitized
    sanitized=$(echo "$title" | tr '[:upper:]' '[:lower:]' | sed 's/[][]//g; s/://g; s/  */-/g; s/^-//; s/-$//')
    local output_path="${SOLANA_DIR}/deployments/${sanitized}.json"

    log_info "Fetching '${title}' from 1Password..." >&2
    op document get "$title" --vault "$OP_VAULT" --out-file "$output_path" --force >/dev/null 2>&1 || {
        log_error "Failed to fetch '${title}' from 1Password vault '${OP_VAULT}'"
        exit 1
    }

    TEMP_KEYPAIR_FILES+=("$output_path")
    echo "$output_path"
}

resolve_upgrade_authority() {
    log_step "Fetching upgrade authority from 1Password"
    UPGRADE_AUTHORITY_KEYPAIR_PATH=$(fetch_keypair_from_op "[Mainnet] Upgrade Authority: Solana")
    UPGRADE_AUTHORITY_PUBKEY=$(solana-keygen pubkey "$UPGRADE_AUTHORITY_KEYPAIR_PATH")
    log_info "Upgrade authority: $UPGRADE_AUTHORITY_PUBKEY"
}

verify_one() {
    local display="$1"
    local library="$2"
    local config_key="$3"

    log_step "Verify ${display}"

    local program_id
    program_id=$(jq -r ".chains[\"${CHAIN}\"].contracts[\"${config_key}\"].address // empty" "$CHAINS_INFO_FILE")

    if [[ -z "$program_id" ]]; then
        log_warn "${display} not found in mainnet.json — skipping"
        return
    fi

    log_info "Program ID:        ${program_id}"
    log_info "Library:           ${library}"
    log_info "Commit:            ${COMMIT_HASH}"

    DOCKER_DEFAULT_PLATFORM=linux/amd64 solana-verify verify-from-repo \
        -u "$RPC_URL" \
        -k "$UPGRADE_AUTHORITY_KEYPAIR_PATH" \
        --program-id "$program_id" \
        --library-name "$library" \
        --commit-hash "$COMMIT_HASH" \
        --base-image "$BASE_IMAGE" \
        --skip-prompt \
        $SKIP_BUILD_FLAG \
        "$REPO_URL" \
        -- --features "$FEATURE" --no-default-features

    log_step "Submitting OtterSec remote build for ${display}"
    solana-verify remote submit-job \
        --program-id "$program_id" \
        --uploader "$UPGRADE_AUTHORITY_PUBKEY" \
        -u "$RPC_URL"
}

# =============================================================================
# Main
# =============================================================================

main() {
    log_step "Solana Mainnet Verification"
    log_info "RPC URL:     ${RPC_URL}"
    log_info "Commit:      ${COMMIT_HASH}"
    [[ -n "$ONLY" ]] && log_info "Only:        ${ONLY}"
    echo ""

    check_prerequisites
    resolve_upgrade_authority

    for entry in "${VERIFY_PROGRAMS[@]}"; do
        IFS='|' read -r display library config_key <<< "$entry"

        if [[ -n "$ONLY" ]]; then
            local short
            short=$(echo "$display" | tr '[:upper:] ' '[:lower:]-')
            [[ "$short" == "$ONLY" ]] || continue
        fi

        verify_one "$display" "$library" "$config_key"
    done

    log_step "Done!"
}

main "$@"
