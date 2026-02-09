#!/usr/bin/env bash
set -uo pipefail

# Load .env if present
if [ -f "$(dirname "$0")/../.env" ]; then
    set -a
    source "$(dirname "$0")/../.env"
    set +a
fi

if [ -z "${PRIVATE_KEY:-}" ]; then
    echo "Error: PRIVATE_KEY is not set."
    exit 1
fi

echo "=== Transfer 1: monad-3 -> berachain (token 0xdae7, origin: monad-3) ==="
ts-node evm/its.js interchain-transfer \
    --destinationChain berachain \
    --tokenId 0xdae74b5fc5709f2a11523fd0a6e6945b0896ac22308cbac6f2e9a3b3407c8de2 \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n monad-3 \
    --env testnet \
    -y

echo ""
echo "=== Transfer 2: berachain -> monad-3 (token 0xe94d, origin: berachain) ==="
ts-node evm/its.js interchain-transfer \
    --destinationChain monad-3 \
    --tokenId 0xe94d11413d745305775b6666376fda16b8a6b0b3c001d373e03b4a957862395c \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n berachain \
    --env testnet \
    -y

echo ""
echo "=== Transfer 3: plume -> hyperliquid ==="
ts-node evm/its.js interchain-transfer \
    --destinationChain hyperliquid \
    --tokenId 0xabfc59828fa9d3b828b014be70917cd683becba3f5e0f0d4a7f7560882f74bbc \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n plume \
    --env testnet \
    -y

echo ""
echo "=== Transfer 4: hyperliquid -> plume ==="
ts-node evm/its.js interchain-transfer \
    --destinationChain plume \
    --tokenId 0xfaff57ede4145c94f34a2dbb32edb94136a7323af86d6d0d0d8413af8a73ebff \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n hyperliquid \
    --env testnet \
    -y

echo ""
echo "=== Done ==="
