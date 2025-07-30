#!/bin/bash

# Programs at `solana/programs` have fixed id's which are not going to be the same when you deploy the program locally. In order
# to fix this situation, we have the `./prepare_test.sh` script, that will do everything for you.

# This script is used to:
#
# 1. Deploy the axelar programs to the solana-test-validator
# 2. Replace the program ids in the bindings and programs rust files
# 3. Build the programs with the new ids
# 4. Deploy the programs again with the new ids

SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_PATH=$SCRIPT_PATH/..

GATEWAY_CODE_ID="gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp"
ITS_CODE_ID="itsqybuNsChBo3LgVhCWWnTJVJdoVTUJaodmqQcG6z7"
MEMO_CODE_ID="memPJFxP6H6bjEKpUSJ4KC7C4dKAfNE3xWrTpJBKDwN"

cd $ROOT_PATH

# Deploy programs and get their ids
echo -e "\e[32mDeploy programs\e[0m"

GATEWAY_ID=$(solana program deploy target/deploy/axelar_solana_gateway.so --program-id target/deploy/axelar_solana_gateway-keypair.json | cut -d' ' -f3)
ITS_ID=$(solana program deploy target/deploy/axelar_solana_its.so --program-id target/deploy/axelar_solana_its-keypair.json | cut -d' ' -f3)
MEMO_PROGRAM_ID=$(solana program deploy target/deploy/axelar_solana_memo_program.so --program-id target/deploy/axelar_solana_memo_program-keypair.json | cut -d' ' -f3)

# Print the program IDs
echo -e "\e[32mDeployed Program IDs\e[0m"

echo "Gateway ID: $GATEWAY_ID"
echo "ITS ID: $ITS_ID"
echo "Memo Program ID: $MEMO_PROGRAM_ID"

# Replace the program IDs in files
echo -e "\e[32mReplacing program IDs in files\e[0m"

sed -i "s/$GATEWAY_CODE_ID/$GATEWAY_ID/g" programs/axelar-solana-gateway/src/lib.rs
sed -i "s/$ITS_CODE_ID/$ITS_ID/g" programs/axelar-solana-its/src/lib.rs
sed -i "s/$MEMO_CODE_ID/$MEMO_PROGRAM_ID/g" programs/axelar-solana-memo-program/src/lib.rs
sed -i "s/$GATEWAY_CODE_ID/$GATEWAY_ID/g" bindings/generated/axelar-solana-gateway/src/program.ts
sed -i "s/$ITS_CODE_ID/$ITS_ID/g" bindings/generated/axelar-solana-its/src/program.ts
sed -i "s/$MEMO_CODE_ID/$MEMO_PROGRAM_ID/g" bindings/generated/axelar-solana-memo-program/src/program.ts

# Ensure the latest version of the contract is built
echo -e "\e[32mBuilding programs with new id's\e[0m"

cd programs/axelar-solana-memo-program && cargo build-sbf
cd ../axelar-solana-gateway && cargo build-sbf
cd ../axelar-solana-its && cargo build-sbf
cd $ROOT_PATH

# Deploy programs again

echo -e "\e[32mDeploy programs again with new id's\e[0m"

solana program deploy target/deploy/axelar_solana_gateway.so --program-id target/deploy/axelar_solana_gateway-keypair.json
solana program deploy target/deploy/axelar_solana_its.so --program-id target/deploy/axelar_solana_its-keypair.json
solana program deploy target/deploy/axelar_solana_memo_program.so --program-id target/deploy/axelar_solana_memo_program-keypair.json
