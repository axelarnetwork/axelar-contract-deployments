# GMP Demo

This document provides a step-by-step guide to execute GMP flow on the SUI network.

## Prerequisites

- Deploy gateway contract `node sui/deploy-gateway.js --signers wallet`
- Deploy gas service contract `node sui/deploy-contract.js GasService`
- Deploy test contract `node sui/deploy-test.js`

## Usage

### Send Command (Outgoing)

Send a messsage from SUI to the destination chain.

Example:
```bash
# node sui/gmp.js sendCall <destChain> <destContractAddress> <feeAmount> <payload>
node sui/gmp.js sendCall ethereum 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05 0.1 0x1234
```

Note:
- `feeAmount` is the amount of SUI coins to be sent to the destination chain. The amount is in full units. For example, `0.1` means 0.1 SUI coins.

### Execute Command (Incoming)

Execute a message from the source chain at SUI application module.

1. Approve the incoming message

```bash
#### sui/gateway.js approve --proof ...... <source> <messageId> <sourceAddress> <destinationId> <payloadHash>
node sui/gateway.js approve --proof wallet ethereum 0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-2 0x4F4495243837681061C4743b74B3eEdf548D56A5 0x6ce0d81b412abca2770eddb1549c9fcff721889c3aab1203dc93866db22ecc4b 0x56570de287d73cd1cb6092bb8fdee6173974955fdef345ae579ee9f475ea7432
```

Note:
- `destinationId` is the channel id of test module. It can be retrieved from test module deployment output.
- `payloadHash` is the keccak256 hash of the payload. The payloadHash in the example `(0x565...7432)` is the hash of `0x1234`.

2. Execute the incoming message

```bash
#### sui/gmp.js execute <source> <messageId> <sourceAddress> <payload>
node sui/gmp.js execute ethereum 0x32034b47cb29d162d9d803cc405356f4ac0ec07fe847ace431385fe8acf3e6e5-2 0x4F4495243837681061C4743b74B3eEdf548D56A5 0x1234
```

Note:
- `source`, `sourceAddress` and `messageId` needed to be matched with the approve command.
- `payload` must be associated with the `payloadHash` in the approve command.
