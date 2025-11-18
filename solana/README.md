# Axelar Solana CLI

A comprehensive CLI tool for interacting with Axelar's Solana programs, including the Gateway, Gas Service, Interchain Token Service (ITS), Governance, Operators, and Memo programs.


## Setup

To begin using the cli. You must clone the [Axelar Amplifier Solana](https://github.com/axelarnetwork/axelar-amplifier-solana) repository into the Solana directory in this repository. 

You can then `cd` into the `axelar-amplifier-solana` and run `cargo build` 


### Environment Variables

In the cloned `axelar-amplifier-solana` repo you will need a `.env` file that contain 

- `SOLANA_PRIVATE_KEY`: Default keypair path for the `--fee-payer`
- `ENV`: The Axelar environment you will interact with
- `CLUSTER`: The Solana cluster you will interact with 
- `CHAIN`: The name of the chain you are interacting with

Environment Example

```bash
ENV="devnet-amplifier"
CLUSTER="devnet"
SOLANA_PRIVATE_KEY=[010, 011, 012, 013, 014, 015 etc.]  #(64-byte Ed25519 keypair)
CHAIN="solana"
```


## GMP

The GMP module provides a simplified interface for sending cross-chain messages.

**Note:** All commands below assume you're running from the repository root. 

```bash
DESTINATION_ACCOUNT="3kKbQ5zXpzeigQLcw82durTRdhnQU7AjfvFhpjbbC8W6:false:true" # Account for execution: doesn't sign, can be modified
PAYLOAD="48656C6C6F21" # "Hello!" in hex
DESTINATION_CHAIN="flow"
DESTINATION_ADDRESS="0x5795699DBBeEbE8e7DB0118A40944Ad8c4e9Dfdc" # Contract addr on destination chain



# 1. Build an Axelar-formatted message payload
GMP_PAYLOAD=$(solana/cli misc build-axelar-message \
  --accounts "$DESTINATION_ACCOUNT" \
  --payload "$PAYLOAD")


# 2. Send the GMP message
solana/cli send \
  --fee-payer ~/.config/solana/id.json \
  gmp send \
  --destination-chain "$DESTINATION_CHAIN" \
  --destination-contract-address "$DESTINATION_ADDRESS" \
  --payload "$GMP_PAYLOAD"
```
