# Cosmwasm Multisig v2.3.0

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @sdavidson1177 <solomon@interoplabs.io>         |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | -                     | TBD        |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |



[Release](https://github.com/axelarnetwork/axelar-amplifier/tree/multisig-v2.3.0)

## Background

Changes in this release:

1. Multisig stores the coordinator address. This address is given when the multisig contract is instantiated. This allows the multisig to give the coordinator permission to execute messages (such as when authorizing callers).
2. Added the `AuthorizedCallers` endpoint. This allows the authorized callers for any given chain to be queried.

## Deployment

- This rollout upgrades the amplifier multisig contract from `v2.1.0` to `v2.3.0`
- State migration is required. The multisig must be supplied with the coordinator's address

1. Upload new Multisig contract

| Network          | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                | `DEPOSIT_VALUE` |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------- |
| devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/> `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
| stagenet         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm`<br/>`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/>`axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
| testnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2`<br/>`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/>`axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |
| mainnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2`<br/>`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/>`axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |

```bash
ts-node cosmwasm/submit-proposal.js store -c Multisig -t "Upload Multisig contract v2.3.0" -d "Upload Multisig contract v2.3.0" -r $RUN_AS_ACCOUNT --deposit $DEPOSIT_VALUE --instantiateAddresses $INIT_ADDRESSES --version 2.2.0
```

2. Upgrade Multisig contract

Provide coordinator address to the multisig.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c Multisig \
  -t "Migrate Multisig to v2.3.0" \
  -d "Multisig to v2.3.0" \
  --msg "{\"coordinator\": \"$COORDINATOR_ADDRESS\"}" \
  --fetchCodeId \
  --deposit $DEPOSIT_VALUE
```

## Checklist

Verify multisig contract version

```bash
ts-node cosmwasm/contract.ts info --contract Multisig -e $ENV
```
Expected output

```bash
{contract: 'multisig', version: '2.3.0'}
```

Verify coordinator address stored on multisig

```bash
axelard q wasm contract-state raw --ascii $MULTISIG_ADDRESS 'config' -o json | jq -r '.data' | base64 -d | jq -r '.coordinator'
```

Expected output

```bash
$COORDINATOR_ADDRESS
```

Ensure coordinator address match predicted one.

```bash
cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.Coordinator.address" | tr -d '"' | grep $COORDINATOR_ADDRESS
```
