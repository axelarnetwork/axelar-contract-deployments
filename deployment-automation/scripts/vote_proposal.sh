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

namespaces=$(kubectl get pods -A | grep "${identifier}-"validator | awk '{print $1}')
for ns in $namespaces; do
    pod=$(kubectl get pods -n $ns | grep "axelar-core-node-validator" | awk '{print $1}')
    echo "Submitting vote for proposal $proposal_id on pod $pod"
    output=$(kubectl exec -n $ns -it $pod -c axelar-core-node -- /bin/sh -c "echo \"\$KEYRING_PASSWORD\" | axelard tx gov vote $proposal_id yes --from validator --gas auto --gas-adjustment 1.4")
    echo "$output"
done