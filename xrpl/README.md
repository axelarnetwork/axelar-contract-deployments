
# XRPL deployments

## Installation

Install npm dependencies.

```sh
npm ci
```

Create a new XRPL keypair.

```bash
node xrpl/generate-wallet.js
```

Set `PRIVATE_KEY` in `.env` to the generated wallet's `seed` value.

Devnet and testnet funds can be obtained via the `faucet.js` script:

```bash
node xrpl/faucet.js -e devnet-amplifier -n xrpl-test-1
```

## Deployments

### XRPL multisig account

Deploy a new XRPL multisig account (the equivalent of the edge AxelarGateway on XRPL):

```bash
node xrpl/deploy-multisig.js -e <env> -n <chain-name> --initialSigners <xrpl-addresses>
```

### CosmWasm contracts

Use the [CosmWasm instructions](../cosmwasm/README.md) as reference to deploy the XRPLVotingVerifier, XRPLGateway, and XRPLMultisigProver contracts to Axelar:

1. Compile the contracts in the [Common Prefix fork of the Amplifier repo](https://github.com/commonprefix/axelar-amplifier/tree/xrpl) using the [rust optimizer](https://github.com/CosmWasm/rust-optimizer) for cosmwasm.

2. Add a `contracts` object to the `axelar` section of your config with the XRPL-specific CosmWasm contract names and instantiation parameters.

```bash
XRPL_RPC_URL="https://s.devnet.rippletest.net:51234"
XRPL_MULTISIG_ADDRESS="rGAbJZEzU6WaYv5y1LfyN7LBBcQJ3TxsKC"
XRPL_AVAILABLE_TICKETS=$(curl -s -X POST $XRPL_RPC_URL -d '{
    "method": "account_objects",
    "params": [{
        "limit": 1000,
        "account": "'$XRPL_MULTISIG_ADDRESS'",
        "ledger_index": "validated",
        "type": "ticket"
    }]
  }' -H "Content-Type: application/json" | jq '.result.account_objects | map(select(.LedgerEntryType == "Ticket") | .TicketSequence)' -c)
XRPL_LAST_ASSIGNED_TICKET_NUMBER=$(echo $XRPL_AVAILABLE_TICKETS | jq 'min | . - 1' -c)
XRPL_NEXT_SEQUENCE_NUMBER=$(curl -s -X POST $XRPL_RPC_URL -d '{
    "method": "account_info",
    "params": [{
        "account": "'$XRPL_MULTISIG_ADDRESS'",
        "ledger_index": "current",
        "strict": true
    }]
  }' -H "Content-Type: application/json" | jq '.result.account_data.Sequence')
```

```json
  "axelar": {
    "contracts": {
      "XrplVotingVerifier": {
        "xrpl-test-1": {
          "governanceAddress": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
          "serviceName": "validators",
          "votingThreshold": [
            "2",
            "3"
          ],
          "blockExpiry": 5,
          "confirmationHeight": 1,
        },
      },
      "XrplGateway": {
        "xrpl-test-1": {
          "governanceAddress": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
          "adminAddress": "axelar1lsasewgqj7698e9a25v3c9kkzweee9cvejq5cs"
        },
      },
      "XrplMultisigProver": {
        "xrpl-test-1": {
          "governanceAddress": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
          "adminAddress": "axelar1lsasewgqj7698e9a25v3c9kkzweee9cvejq5cs",
          "signingThreshold": [
            "2",
            "3"
          ],
          "serviceName": "validators",
          "verifierSetDiffThreshold": 1,
          "xrplFee": 300,
          "ticketCountThreshold": 5,
          "availableTickets": $XRPL_AVAILABLE_TICKETS,
          "nextSequenceNumber": $XRPL_NEXT_SEQUENCE_NUMBER,
          "lastAssignedTicketNumber": $XRPL_LAST_ASSIGNED_TICKET_NUMBER
        },
      }
    },

    "rpc": [rpc],
    "tokenSymbol": "amplifier",
    "gasPrice": "0.00005uamplifier",
    "gasLimit": 5000000
  }
```

```bash
node cosmwasm/deploy-contract.js upload-instantiate -a <path-to-artifacts> -c "XrplVotingVerifier" --instantiate2 -e devnet-amplifier -m $MNEMONIC -n xrpl-test-1

node cosmwasm/deploy-contract.js upload-instantiate -a <path-to-artifacts> -c "XrplGateway" --instantiate2 -e devnet-amplifier -m $MNEMONIC -n xrpl-test-1

node cosmwasm/deploy-contract.js upload-instantiate -a <path-to-artifacts> -c "XrplMultisigProver" --instantiate2 -e devnet-amplifier -m $MNEMONIC -n xrpl-test-1
```

## GMP/ITS Transfers

GMP and/or ITS transfers can be performed via the `transfer.js` script:

```bash
node xrpl/transfer.js send [token] [amount] [destination-chain] [destination-address] --gas-fee-amount [gas-fee-amount] --payload [payload]
```

Here's an example of a token transfer that also performs GMP:

```bash
node xrpl/transfer.js -e devnet-amplifier -n xrpl-test-1 XRP 1 xrpl-evm-sidechain 0x0A90c0Af1B07f6AC34f3520348Dbfae73BDa358E --data 0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000e474d5020776f726b7320746f6f3f000000000000000000000000000000000000
```
