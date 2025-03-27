
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
node xrpl/faucet.js -e devnet-amplifier -n xrpl
```

## XRPL Multisig Account Deployment

Deploy a new XRPL multisig account (the equivalent of the edge AxelarGateway on XRPL):

```bash
node xrpl/deploy-multisig.js -e <env> -n <chain-name> --initialSigner <xrpl-address>
```

This will transform the environment wallet into an XRPL multisig account and configure it appropriately.

## Rotate XRPL Multisig Signers

Rotate the XRPL multisig account's signer set (via a `SignerListSet` transaction):

```bash
node xrpl/rotate-signers.js -e <env> -n <chain-name> --signerPublicKeys <signer-public-keys> --signerWeights <signer-weights> --quorum <quorum>
```

The environment wallet must be the initial signer of the multisig account, with enough weight to reach quorum.

Here's an example signer set rotation:

```bash
node xrpl/rotate-signers.js -e testnet -n xrpl --signerPublicKeys 028E425D6F75EC61C8568B7E1C29D3085E210A90A0CE6491E7A249747D34431F6C 02D904B083B855A5AE1DAB39ACE60227E110E0490AAA74DE18F5806121369DBB48 02F77F629E38433F6D2CE5EE46B7E8E1724444163FB08B99CF2C1B117A0E8578F1 0285737FE8BA5D8E8F2A10CB39E814D5E72DADF8FF05BDFABCCF1EF20C51279EC8 --signerWeights 1 1 1 1 --quorum 3
```

## ITS Interchain Transfers

Interchain token transfers can be performed via the `interchain-transfer.js` script:

```bash
node xrpl/interchain-transfer.js -e <env> -n <source-chain> [token] [amount] [destination-chain] [destination-address] --gasFeeAmount <gas-fee-amount> --payload <payload>
```

Here's an example of an interchain transfer that also performs GMP:

```bash
node xrpl/interchain-transfer.js -e devnet-amplifier -n xrpl XRP 1 xrpl-evm-sidechain 0x0A90c0Af1B07f6AC34f3520348Dbfae73BDa358E --gasFeeAmount 0 --payload 0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000e474d5020776f726b7320746f6f3f000000000000000000000000000000000000
```

## Broadcast Raw Transaction Blob

Broadcast a raw, signed transaction blob (e.g., a completed Multisig Prover proof) to XRPL:

```bash
node xrpl/broadcast.js -e <env> -n <chain-name> [blob]
```

Here's an example:

```bash
node xrpl/broadcast.js -e testnet -n xrpl 12000C220000000024004C8592201B005199CE20230000000168400000000000000C730081140ACC2A59A96BEC485157E9841606C9AD844BA207F3E0107321028E425D6F75EC61C8568B7E1C29D3085E210A90A0CE6491E7A249747D34431F6C74463044022076E3EE68B9653C592D50DEFC885A6957A14B5924ECE3F61D42CD0056299DB428022029DE562319B3A4BC611B9EF1B0EE512438A48B25FAAD3D22E354BE14C780BE338114462757093D0F1DAD8F75922B19BE3AA5D9AC6B9DE1F1F4EB1300018114462757093D0F1DAD8F75922B19BE3AA5D9AC6B9DE1EB1300018114026EA26A49F2D5F2752BB15FE72C5812B7863824E1EB13000181141A1CC32118C7C7A156A889841A5E49A210B71342E1EB13000181141E88A08547159640ECCB53A52587987006D81A10E1EB130001811444B9500EB378468F8585F2275EABCC41252FC7CFE1EB130001811449E6787D8AE8AC5B1760B98585466EC09CF1E590E1EB13000181144A24EFF7061CE3D81B5C3FD6C1C74542B8055E64E1EB13000181144D85D4DC50B6CAA6797C4988D3B9ED9F9FB758A7E1EB13000181144E5C67BB4948F5959FEB1B7D23CB9A626D558F03E1EB13000181147462D3DEEF5F2EA11D3303C5ACF365BFB31CCA2FE1EB13000181148B71CE3A3EF0FAD3DF56F2B40673814E41B4287CE1EB13000181148D83D8D3F03A40B3A053F7C749652450720512C2E1EB13000181149196F13C2EFBAF8F8883F462FA98AD1041B4F3F2E1EB1300018114944FF4E33F562F654C37D7003D51E33F2D314D2EE1EB13000181149D9F9F164FC5B2401DF537E7DC13B4FCDC9A11F2E1EB1300018114AA9B0641CC13BF6182E3263BC4E7B7C7E6152654E1EB1300018114B4B3D25493774F58966937A098A07690691F2681E1EB1300018114C794445AA5F6E84A585A9738CFD2276AE89B0346E1EB1300018114CD17B9A2AA9EB6F6734E026325F9BE73D4146101E1EB1300018114D3D541E7C5DFE6CA530E1CBB29E2C1FF884D2B0CE1EB1300018114E3875AEFB4FEC31D93A1DECAB733A70DC4203530E1EB1300018114F1F517C9E934ACA53AC90FDB4187F03DEA735CDDE1EB1300018114FDDB12153E54A22CB125A3FDEF3FD650F0AD7C41E1F1
```

## Decode XRPL Address

Convert an XRPL Address (aka Account ID) into raw bytes (e.g., to be used when specifying an XRPL destination address on ITS):

```bash
node xrpl/decode-address.js [account-id]
```

Here's an example:

```bash
node xrpl/decode-address.js r9m9uUCAwMLSnRryXYuUB3cGXojpRznaAo
# Account ID raw bytes: 0x601abcea746a193f32ed460dd933f15441142d6b
```

## Decode Raw Transaction Blob

Deserialize a raw transaction blob into a readable transaction object:

```bash
node xrpl/decode-tx-blob.js <tx-blob>
```

Here's a truncated example:

```bash
node xrpl/decode-tx-blob.js 120000220000000024000000002029004c6ce7614[...]738623034353436656239322d3732393935e1f1
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

## Create a Trust Line

Create a trust line between your account and a token issuer (via a TrustSet transaction).

```bash
node xrpl/trust-set.js -e <env> -n <chain-name> --multisign --account <account> [token-currency] [token-issuer-address]
```

Here's an example:

```bash
node xrpl/trust-set.js -e devnet-amplifier -n xrpl XYZ r4DVHyEisbgQRAXCiMtP2xuz5h3dDkwqf1
```

## Modify Account Properties

Modify an account's properties (via an AccountSet transaction).

```bash
node xrpl/account-set.js -e <env> -n <source-chain> --multisign --account <account> --transferRate <transfer-rate> --tickSize <tick-size> --domain <domain> --flag <flag>
```

Here's an example:

```bash
node xrpl/account-set.js -e testnet -n xrpl --multisign --account rsCPY4vwEiGogSraV9FeRZXca6gUBWZkhg --transferRate 0 --tickSize 6 --domain axelar.foundation --flag 14
```
