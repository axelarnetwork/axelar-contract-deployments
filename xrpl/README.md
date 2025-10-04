# XRPL deployments

## Table of Contents

-   [Installation](#installation)
-   [Deployment](#deployment)
-   [Contract Interactions](#contract-interactions)
-   [Helpers](#helpers)

## Installation

1. Install npm dependencies.

```sh
npm ci && npm run build
```

2. Create a new XRPL keypair via the [`generate-wallet.js` script](#generate-wallet).

3. Set `PRIVATE_KEY` in `.env` to the generated wallet's `seed` value.

4. Claim devnet and testnet funds via the [`faucet.js` script](#claim-funds-from-faucet), if applicable.

5. Set `ENV` and `CHAIN` in `.env` to the environment ("devnet-amplifier", "stagenet", "testnet", or "mainnet") and chain name (`xrpl`), respectively, to avoid having to specify these on every command.

## Deployment

### XRPL Multisig Account Deployment

Deploy a new XRPL multisig account (the equivalent of the edge AxelarGateway on XRPL):

```bash
ts-node xrpl/deploy-multisig.js -e <env> -n <chain-name> --initialSigner <xrpl-address>
```

This will transform the environment wallet into an XRPL multisig account and configure it appropriately.

### Rotate XRPL Multisig Signers

Rotate the XRPL multisig account's signer set (via a `SignerListSet` transaction):

```bash
ts-node xrpl/rotate-signers.js -e <env> -n <chain-name> --signerPublicKeys <signer-public-keys> --signerWeights <signer-weights> --quorum <quorum>
```

The environment wallet must be the initial signer of the multisig account, with enough weight to reach quorum.

Here's an example signer set rotation:

```bash
ts-node xrpl/rotate-signers.js -e testnet -n xrpl --signerPublicKeys 028E425D6F75EC61C8568B7E1C29D3085E210A90A0CE6491E7A249747D34431F6C 02D904B083B855A5AE1DAB39ACE60227E110E0490AAA74DE18F5806121369DBB48 02F77F629E38433F6D2CE5EE46B7E8E1724444163FB08B99CF2C1B117A0E8578F1 0285737FE8BA5D8E8F2A10CB39E814D5E72DADF8FF05BDFABCCF1EF20C51279EC8 --signerWeights 1 1 1 1 --quorum 3
```

### Token Deployments

Refer to the XRPL ITS [Local Token Deployment](./docs/deploy-local-token.md), [Remote Token Deployment](./docs/deploy-remote-token.md),  and [Link Token](./docs/link-token.md) guides to enable transferring new tokens between XRPL and remote chains.

## Contract Interactions

Since there's no smart contracts on XRPL, all interactions happen as `Payment` transactions towards the XRPL multisig account.
The XRPL multisig account is used in place of both AxelarGateway and InterchainTokenService.

### ITS Interchain Transfers

Interchain token transfers from XRPL are initiated via a `Payment` transaction that respects the following format:

```js
{
    TransactionType: "Payment",
    Account: user.address, // sender's account address
    Amount: "1000000", // amount of XRP to send, in drops (in this case, 1 XRP), *including* gas fee amount
    // Amount: { // alternatively, an IOU token amount can be specified, when transferring some IOU rather than XRP
    //     currency: "ABC", // IOU's currency code
    //     issuer: "r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1", // IOU issuer's account address
    //     value: "1" // IOU amount to bridge (in this case, 1 ABC.r4DVH), *including* gas fee amount
    // },
    Destination: multisig.address, // Axelar multisig's account address
    Memos: [
        {
            Memo: {
                MemoType: "74797065", // hex("type")
                MemoData: "696e746572636861696e5f7472616e73666572" // hex("interchain_transfer")
            },
        },
        {
            Memo: {
                MemoType: "64657374696e6174696f6e5f61646472657373", // hex("destination_address")
                // recipient's address, without the 0x prefix (in the EVM case), hex-encoded - hex("0A90c0Af1B07f6AC34f3520348Dbfae73BDa358E"), in this case:
                MemoData: "30413930633041663142303766364143333466333532303334384462666165373342446133353845"
            },
        },
        {
            Memo: {
                MemoType: "64657374696E6174696F6E5F636861696E", // hex("destination_chain")
                MemoData: "7872706c2d65766d2d6465766e6574", // destination chain, hex encoded - hex("xrpl-evm-devnet"), in this case
            },
        },
        {
            Memo: {
                MemoType: "6761735f6665655f616d6f756e74", // hex("gas_fee_amount")
                // the amount to be deducted from the total payment amount to cover gas fee -
                // this amount is denominated in the same token that's being transferred
                // (i.e., if you're bridging XRP, this value corresponds to the amount of XRP drops that will be used to cover gas fees,
                // while if you're bridging some IOU, it's the amount of IOU tokens that will be allocated to gas fees)
                MemoData: "30", // amount of tokens to allocate to gas fees, out of the amount being sent to the multisig, hex encoded - hex("0"), in this case
            },
        },
        { // Only include this Memo object when performing a GMP call:
            Memo: {
                MemoType: "7061796c6f6164", // hex("payload")
                // abi-encoded payload/data with which to call the Executable destination contract address:
                MemoData: "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000e474d5020776f726b7320746f6f3f000000000000000000000000000000000000",
            },
        },
    ],
    ...
}
```

Interchain token transfers can be performed via the `interchain-transfer.js` script:

```bash
ts-node xrpl/interchain-transfer.js -e <env> -n <source-chain> [token] [amount] [destination-chain] [destination-address] --gasFeeAmount <gas-fee-amount> --payload <payload>
```

Here's an example of an interchain transfer that also performs GMP:

```bash
ts-node xrpl/interchain-transfer.js -e devnet-amplifier -n xrpl XRP 1 xrpl-evm-sidechain 0x0A90c0Af1B07f6AC34f3520348Dbfae73BDa358E --gasFeeAmount 0 --payload 0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000e474d5020776f726b7320746f6f3f000000000000000000000000000000000000
```

### General Message Passing

GMP contract calls from XRPL are initiated via a `Payment` transaction that respects the following format:

```js
{
    TransactionType: "Payment",
    Account: user.address, // sender's account address
    Amount: "1000000", // amount of XRP used to cover gas fees, in drops (in this case, 1 XRP)
    // Amount: { // alternatively, an IOU token amount can be used to cover gas fees
    //     currency: "ABC", // IOU's currency code
    //     issuer: "r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1", // IOU issuer's account address
    //     value: "1" // IOU amount to allocated to gas fees (in this case, 1 ABC.r4DVH)
    // },
    Destination: multisig.address, // Axelar multisig's account address
    Memos: [
        {
            Memo: {
                MemoType: "74797065", // hex("type")
                MemoData: "63616c6c5f636f6e7472616374" // hex("call_contract")
            },
        },
        {
            Memo: {
                MemoType: "64657374696e6174696f6e5f61646472657373", // hex("destination_address")
                // // destination smart contract address, without the 0x prefix (in the EVM case), hex-encoded -
                // hex("0A90c0Af1B07f6AC34f3520348Dbfae73BDa358E"), in this case:
                MemoData: "30413930633041663142303766364143333466333532303334384462666165373342446133353845"
            },
        },
        {
            Memo: {
                MemoType: "64657374696E6174696F6E5F636861696E", // hex("destination_chain")
                MemoData: "7872706c2d65766d2d6465766e6574", // destination chain, hex encoded - hex("xrpl-evm-devnet"), in this case
            },
        },
        {
            Memo: {
                MemoType: "7061796c6f6164", // hex("payload")
                // abi-encoded payload/data with which to call the Executable destination contract address:
                MemoData: "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000e474d5020776f726b7320746f6f3f000000000000000000000000000000000000",
            },
        },
    ],
    ...
}
```

Pure GMP (without token transfers) can be performed via the `call-contract.js` script:

```bash
ts-node xrpl/call-contract.js -e <env> -n <source-chain> [destination-chain] [destination-address] --gasFeeAmount <gas-fee-amount> --gasFeeToken <gas-fee-token> --payload <payload>
```

Here's an example:

```bash
ts-node xrpl/call-contract.js -e devnet-amplifier -n xrpl-dev xrpl-evm-devnet 0x2F630a1CE68d76ff2113D2F3AE8FB64Abf7d3804 --gasFeeAmount 1 --gasFeeToken XRP --payload 0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000B48656C6C6F20576F726C64000000000000000000000000000000000000000000 -y
```

### Add Gas

A pending GMP/ITS message can be topped up from XRPL via a `Payment` transaction that respects the following format:

```js
{
    TransactionType: "Payment",
    Account: user.address, // sender's account address
    Amount: "1000000", // amount of XRP, in drops (in this case, 1 XRP), to top-up gas fees with
    // Amount: { // alternatively, an IOU token amount can be used to top up gas fees
    //     currency: "ABC", // IOU's currency code
    //     issuer: "r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1", // IOU issuer's account address
    //     value: "1" // IOU amount to top up gas fees with (in this case, 1 ABC.r4DVH)
    // },
    Destination: multisig.address, // Axelar multisig's account address
    Memos: [
        {
            Memo: {
                MemoType: "74797065", // hex("type")
                MemoData: "6164645f676173" // hex("add_gas")
            },
        },
        {
            Memo: {
                MemoType: "6d73675f6964", // hex("msg_id")
                // message ID of the pending GMP or ITS transaction whose gas to top up, hex encoded - hex("c7c653d2df83622c277da55df7fe6466098f5bc2e466e1251f42772d07016c8c"), in this case:
                MemoData: "63376336353364326466383336323263323737646135356466376665363436363039386635626332653436366531323531663432373732643037303136633863"
            },
        },
    ],
    ...
}
```

You can use the `add-gas.js` script to top-up an ITS or GMP message's gas amount:

```bash
ts-node xrpl/add-gas.js -e <env> -n <source-chain> --token <token> --amount <amount> --msgId <msg-id>
```

Here's an example:

```bash
ts-node xrpl/add-gas.js -e devnet-amplifier -n xrpl-dev --amount 0.1 --token XRP --msgId C7C653D2DF83622C277DA55DF7FE6466098F5BC2E466E1251F42772D07016C8C
```

### Add XRP to Fee Reserve

A fee reserve is used to cover the XRPL multisig account's reserve requirements as well as
Proof transaction fees (i.e., gas fees of transactions generated by the XRPL Multisig Prover).
These fee reserve top-ups are intended to be performed by the XRPL relayer.

XRP fee reserve top-ups are initiated from XRPL via a `Payment` transaction that respects the following format:

```js
{
    TransactionType: "Payment",
    Account: user.address, // sender's account address
    Amount: "1000000", // amount of XRP, in drops (in this case, 1 XRP), to top-up the fee reserve with
    Destination: multisig.address, // Axelar multisig's account address
    Memos: [
        {
            Memo: {
                MemoType: "74797065", // hex("type")
                MemoData: "6164645f7265736572766573" // hex("add_reserves")
            },
        },
    ],
    ...
}
```

You can use the `add-reserves.js` script to top up the XRP fee reserve:

```bash
ts-node xrpl/add-reserves.js -e <env> -n <chain-name> --amount <amount>
```

Here's an example:

```bash
ts-node xrpl/add-reserves.js -e devnet-amplifier -n xrpl-dev --amount 10
```

## Helpers

These scripts are intended to help with interactions with the XRPL.

### Generate Wallet

Generate a new XRPL wallet:

```bash
ts-node xrpl/generate-wallet.js
```

Set `SEED` in `.env` to the generated `seed` to use this wallet for performing other actions.

### Claim Funds from Faucet

Claim funds using the XRPL faucet:

```bash
ts-node xrpl/faucet.js -e <env> -n <chain-name> --minBalance <min-balance> --amount <amount> --recipient <recipient>
```

Here's an example:

```bash
ts-node xrpl/faucet.js --minBalance 100 --amount 100
```

### Broadcast Raw Transaction Blob

Broadcast a raw, signed transaction blob (e.g., a completed Multisig Prover proof) to XRPL:

```bash
ts-node xrpl/broadcast.js -e <env> -n <chain-name> [blob]
```

Here's an example:

```bash
ts-node xrpl/broadcast.js -e testnet -n xrpl 12000C220000000024004C8592201B005199CE20230000000168400000000000000C730081140ACC2A59A96BEC485157E9841606C9AD844BA207F3E0107321028E425D6F75EC61C8568B7E1C29D3085E210A90A0CE6491E7A249747D34431F6C74463044022076E3EE68B9653C592D50DEFC885A6957A14B5924ECE3F61D42CD0056299DB428022029DE562319B3A4BC611B9EF1B0EE512438A48B25FAAD3D22E354BE14C780BE338114462757093D0F1DAD8F75922B19BE3AA5D9AC6B9DE1F1F4EB1300018114462757093D0F1DAD8F75922B19BE3AA5D9AC6B9DE1EB1300018114026EA26A49F2D5F2752BB15FE72C5812B7863824E1EB13000181141A1CC32118C7C7A156A889841A5E49A210B71342E1EB13000181141E88A08547159640ECCB53A52587987006D81A10E1EB130001811444B9500EB378468F8585F2275EABCC41252FC7CFE1EB130001811449E6787D8AE8AC5B1760B98585466EC09CF1E590E1EB13000181144A24EFF7061CE3D81B5C3FD6C1C74542B8055E64E1EB13000181144D85D4DC50B6CAA6797C4988D3B9ED9F9FB758A7E1EB13000181144E5C67BB4948F5959FEB1B7D23CB9A626D558F03E1EB13000181147462D3DEEF5F2EA11D3303C5ACF365BFB31CCA2FE1EB13000181148B71CE3A3EF0FAD3DF56F2B40673814E41B4287CE1EB13000181148D83D8D3F03A40B3A053F7C749652450720512C2E1EB13000181149196F13C2EFBAF8F8883F462FA98AD1041B4F3F2E1EB1300018114944FF4E33F562F654C37D7003D51E33F2D314D2EE1EB13000181149D9F9F164FC5B2401DF537E7DC13B4FCDC9A11F2E1EB1300018114AA9B0641CC13BF6182E3263BC4E7B7C7E6152654E1EB1300018114B4B3D25493774F58966937A098A07690691F2681E1EB1300018114C794445AA5F6E84A585A9738CFD2276AE89B0346E1EB1300018114CD17B9A2AA9EB6F6734E026325F9BE73D4146101E1EB1300018114D3D541E7C5DFE6CA530E1CBB29E2C1FF884D2B0CE1EB1300018114E3875AEFB4FEC31D93A1DECAB733A70DC4203530E1EB1300018114F1F517C9E934ACA53AC90FDB4187F03DEA735CDDE1EB1300018114FDDB12153E54A22CB125A3FDEF3FD650F0AD7C41E1F1
```

### Decode XRPL Address

Convert an XRPL Address (aka Account ID) into raw bytes (e.g., to be used when specifying an XRPL destination address on ITS):

```bash
ts-node xrpl/decode-address.js [account-id]
```

Here's an example:

```bash
ts-node xrpl/decode-address.js r9m9uUCAwMLSnRryXYuUB3cGXojpRznaAo
# Account ID raw bytes: 0x601abcea746a193f32ed460dd933f15441142d6b
```

### Decode Raw Transaction Blob

Deserialize a raw transaction blob into a readable transaction object:

```bash
ts-node xrpl/decode-tx-blob.js <tx-blob>
```

Here's a truncated example:

```bash
ts-node xrpl/decode-tx-blob.js 120000220000000024000000002029004c6ce7614[...]738623034353436656239322d3732393935e1f1
# Decoded transaction: {
#   "TransactionType": "Payment",
#   "Flags": 0,
#   "Sequence": 0,
#   "TicketSequence": 5008615,
#   "Amount": "1000000",
#   "Fee": "5000",
#    [...]
# }
```

### Make a Payment

Perform a token transfer (via a Payment transaction).

```bash
ts-node xrpl/payment.js -e <env> -n <chain-name> --multisign --from <from> --to <to> --amount <amount> --token <token>
```

Here's an example:

```bash
ts-node xrpl/payment.js -e devnet-amplifier -n xrpl-dev --to rNY3vQMxmrKWnAboNtKdthPmqL4TD7ak3m --amount 1 --token "ABC.r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1"
```

### Create a Trust Line

Create a trust line between your account and a token issuer (via a TrustSet transaction).

```bash
ts-node xrpl/trust-set.js -e <env> -n <chain-name> --multisign --account <account> [token-currency] [token-issuer-address]
```

Here's an example:

```bash
ts-node xrpl/trust-set.js -e devnet-amplifier -n xrpl XYZ r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1
```

### Create Tickets

Create tickets (via a TicketCreate transaction).

```bash
ts-node xrpl/ticket-create.js -e <env> -n <chain-name> --multisign --account <account> --ticketCount <ticket-count>
```

Here's an example:

```bash
ts-node xrpl/ticket-create.js -e devnet-amplifier -n xrpl-dev --multisign --account rGAbJZEzU6WaYv5y1LfyN7LBBcQJ3TxsKC --ticketCount 250
```

### Modify Account Properties

Modify an account's properties (via an AccountSet transaction).

```bash
ts-node xrpl/account-set.js -e <env> -n <source-chain> --multisign --account <account> --transferRate <transfer-rate> --tickSize <tick-size> --domain <domain> --flag <flag>
```

Here's an example:

```bash
ts-node xrpl/account-set.js -e testnet -n xrpl --multisign --account rsCPY4vwEiGogSraV9FeRZXca6gUBWZkhg --transferRate 0 --tickSize 6 --domain axelar.foundation --flag 14
```

### Submit Amplifier Proofs

To submit proofs constructed on Amplifier to the XRPL multisig, use the following command:

```bash
ts-node xrpl/submit-proof.js -e <env> -n <chain-name> [session id]
```

### Query XRPL Token ID

Query an XRPL token ID from the `XRPLGateway` contract.

```bash
ts-node xrpl/xrpl-token-id.js -e <env> -n <chain-name> --issuer <token-issuer> --currency <token-currency>
```

Here's an example:

```bash
ts-node xrpl/xrpl-token-id.js -e devnet-amplifier -n xrpl-dev --issuer r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1 --currency XYZ
# Token ID for XYZ.r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1: a7ea6e58bb50cc7f25a9a68a245d5757089b775100509454bc236b56806fc249
```

### Register Local XRPL Token

Add support for an XRPL IOU token.

```bash
ts-node xrpl/register-local-token.js -e <env> -n <chain-name> --issuer <token-issuer> --currency <token-currency>
```

Here's an example:

```bash
ts-node xrpl/register-local-token.js -e devnet-amplifier -n xrpl-dev --issuer r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1 --currency XYZ
```

### Create a Trust Line via the Multisig

Create a trust line between the multisig account and a token issuer (via a TrustSet transaction).

```bash
ts-node xrpl/trust-set-multisig.js -e <env> -n <chain-name> --tokenId <token-id>
```

Here's an example:

```bash
ts-node xrpl/trust-set-multisig.js -e devnet-amplifier -n xrpl-dev --tokenId a7ea6e58bb50cc7f25a9a68a245d5757089b775100509454bc236b56806fc249
```

### Register Remote Token

Add support for a remote token.

```bash
ts-node xrpl/register-remote-token.js -e <env> -n <chain-name> --tokenId <token-id> --currency <token-currency>
```

Here's an example:

```bash
ts-node xrpl/register-remote-token.js -e devnet-amplifier -n xrpl-dev --tokenId d059cbf3cf520f4d96064e094cb5e8fbb501bc4744034a5ca840dc2beb658aaa --currency FOO
```

### Register Token Metadata

Register an XRPL token's metadata on ITS Hub.

```bash
ts-node xrpl/register-token-metadata.js -e <env> -n <chain-name> --issuer <token-issuer> --currency <token-currency>
```

Here's an example:

```bash
ts-node xrpl/register-token-metadata.js -e devnet-amplifier -n xrpl-dev --issuer r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1 --currency XYZ
# Initiated token metadata registration: 69C696A56200BDFB25D7CCB44537239801D69D8B67D8077E2D1012404378A4A0
#
# Message ID: 0x52006f03806f806d680bb8c932072fe3a12f1a171161e8e894f76ff355052d46
#
# Payload: 00000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000f000000000000000000000000000000000000000000000000000000000000002658595a2e72344456487945697362675152415843694d74503278757a35683364446b777166310000000000000000000000000000000000000000000000000000
#
# Token address: 58595a2e72344456487945697362675152415843694d74503278757a35683364446b77716631
```

### Deploy Remote Token

Deploy an XRPL token to a remote chain.

```bash
ts-node xrpl/deploy-remote-token.js -e <env> -n <chain-name> --issuer <token-issuer> --currency <token-currency> --tokenName <token-name> --tokenSymbol <token-symbol> --destinationChain <destination-chain>
```

Here's an example:

```bash
ts-node xrpl/deploy-remote-token.js -e devnet-amplifier -n xrpl-dev --issuer r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1 --currency XYZ --tokenName XYZ.axl --tokenSymbol XYZ.axl --destinationChain flow
# Initiated remote token deployment: FD4CA2F86DAD7B93E434C771FC2805876B6A1032E0714FE97695DFED5D009FC5
#
# Message ID: 0x6e9e04a2443546b7b201965850bcdfef4af303ebd041e12cd63d6273e4b5b6cd
#
# Payload: 0000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000004666c6f770000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000001a7ea6e58bb50cc7f25a9a68a245d5757089b775100509454bc236b56806fc24900000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000f0000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000000758595a2e61786c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000758595a2e61786c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
```
