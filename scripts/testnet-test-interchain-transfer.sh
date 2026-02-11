#!/usr/bin/env bash
set -euo pipefail

# Load .env if present
if [ -f "$(dirname "$0")/../.env" ]; then
    set -a
    source "$(dirname "$0")/../.env"
    set +a
fi

if [ -z "${EVM_PRIVATE_KEY:-}" ]; then
    echo "Error: EVM_PRIVATE_KEY is not set."
    exit 1
fi

if [ -z "${XRPL_PRIVATE_KEY:-}" ]; then
    echo "Error: XRPL_PRIVATE_KEY is not set."
    exit 1
fi

if [ -z "${STELLAR_PRIVATE_KEY:-}" ]; then
    echo "Error: STELLAR_PRIVATE_KEY is not set."
    exit 1
fi

if [ -z "${SUI_PRIVATE_KEY:-}" ]; then
    echo "Error: SUI_PRIVATE_KEY is not set."
    exit 1
fi

echo "=== Checking wallet balances ==="
node scripts/check-wallet-balances.js --env testnet
echo ""

echo "=== Transfer 1: monad-3 -> berachain (token 0xdae7, origin: monad-3) ==="
PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/its.js interchain-transfer \
    --destinationChain berachain \
    --tokenId 0xdae74b5fc5709f2a11523fd0a6e6945b0896ac22308cbac6f2e9a3b3407c8de2 \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n monad-3 \
    --env testnet \
    -y

echo ""
echo "=== Transfer 2: berachain -> monad-3 (token 0xe94d, origin: berachain) ==="
PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/its.js interchain-transfer \
    --destinationChain monad-3 \
    --tokenId 0xe94d11413d745305775b6666376fda16b8a6b0b3c001d373e03b4a957862395c \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n berachain \
    --env testnet \
    -y

echo ""
echo "=== Transfer 3: plume -> hyperliquid ==="
PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/its.js interchain-transfer \
    --destinationChain hyperliquid \
    --tokenId 0xabfc59828fa9d3b828b014be70917cd683becba3f5e0f0d4a7f7560882f74bbc \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n plume \
    --env testnet \
    -y

echo ""
echo "=== Transfer 4: hyperliquid -> plume ==="
PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/its.js interchain-transfer \
    --destinationChain plume \
    --tokenId 0xfaff57ede4145c94f34a2dbb32edb94136a7323af86d6d0d0d8413af8a73ebff \
    --destinationAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --amount 100 \
    -n hyperliquid \
    --env testnet \
    -y

echo ""
echo "=== Transfer 5: xrpl -> xrpl-evm (XRP) ==="
PRIVATE_KEY="$XRPL_PRIVATE_KEY" ts-node xrpl/interchain-transfer.js \
    -e testnet \
    -n xrpl \
    XRP 1 xrpl-evm 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
    --gasFeeAmount 500000 \
    -y

echo ""
echo "=== Transfer 6: xrpl-evm -> xrpl (XRP) ==="
PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/its.js interchain-transfer \
    --destinationChain xrpl \
    --tokenId 0xba5a21ca88ef6bba2bfff5088994f90e1077e2a1cc3dcc38bd261f00fce2824f \
    --destinationAddress rPgTwjrZtcZKNyMaEH82NddRUBNkcX1kz7 \
    --amount 0.5 \
    -n xrpl-evm \
    --env testnet \
    -y

echo ""
echo "=== Transfer 7: stellar -> flow (HBT) ==="
PRIVATE_KEY="$STELLAR_PRIVATE_KEY" ts-node stellar/its.js interchain-transfer \
    0x54fcc6520dc7b9bd8e17300edf1c80b7cf0292260ec14073de6c979103688699 \
    flow 0xba76c6980428A0b10CFC5d8ccb61949677A61233 1000 \
    -e testnet --chain-name stellar-2026-q1-2 \
    -y

echo ""
echo "=== Transfer 8: sui -> flow (HBT16) ==="
PRIVATE_KEY="$SUI_PRIVATE_KEY" ts-node sui/its.js interchain-transfer \
    0xd33596a1c02b6d91e34cd5643e7e2988a6554fc4bff94f4e240decd85e667075 \
    flow 0xba76c6980428A0b10CFC5d8ccb61949677A61233 1 \
    -e testnet -n sui \
    -y

echo ""
echo "=== Transfer 9: flow -> sui (HBT16) ==="
PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/its.js interchain-transfer \
    --destinationChain sui \
    --tokenId 0xd33596a1c02b6d91e34cd5643e7e2988a6554fc4bff94f4e240decd85e667075 \
    --destinationAddress 0x9b8b3a3e2d0bbee851424e84ac84211dfb02f72dd4d8bc136639d6e2e7773d2f \
    --amount 1 \
    -n flow \
    --env testnet \
    -y

echo ""
echo "=== Done ==="
