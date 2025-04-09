#!/bin/bash

identifier="$1"
if [ -z "$1" ]; then
    echo "Usage: $0 <identifier> <proposal_id>"
    exit 1
fi
echo "Identifier: $identifier"

# proposal id
if [ -z "$2" ]; then
    echo "Usage: $0 <identifier> <proposal_id>"
    exit 1
fi
proposal_id="$2"

for i in {0..3}; do
    output=$(kubectl exec -n $identifier -it validator-$i -c core -- /bin/sh -c "echo \"\$KEYRING_PASSWORD\" | axelard tx gov vote $proposal_id yes --from validator --gas auto --gas-adjustment 1.4")
    echo "$output"
done