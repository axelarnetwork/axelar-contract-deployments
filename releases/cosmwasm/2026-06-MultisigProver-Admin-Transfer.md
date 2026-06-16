# MultisigProver Admin Transfer

|                | **Owner** |
| -------------- | --------- |
| **Created By** | rista404  |
| **Deployment** | rista404  |

| **Network** | **Deployment Status** | **Date** |
| ----------- | --------------------- | -------- |
| **Mainnet** | Pending               | TBD      |

## Background

Transfer the `adminAddress` for every chain-specific `MultisigProver` contract whose current admin is
`axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj`.

This transfer must be executed through Axelar governance. `MultisigProver` supports `update_admin`, but the message is
guarded by the `Governance` permission, not the `Admin` permission. The current admin EOA cannot rotate this role
directly.

Each included governance message updates one `MultisigProver` instance:

```json
{
  "update_admin": {
    "new_admin_address": "axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly"
  }
}
```

## Scope

- Environment: `mainnet`
- Source admin: `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj`
- Target admin: `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly`
- Contract set: all configured chain-specific `MultisigProver` instances whose live admin matches the source admin

## Preconditions

- `.env` is configured for mainnet Axelar operations, including `ENV=mainnet`, `AXELAR_RPC`, and proposer signing
  configuration.
- Confirm each included `MultisigProver` has the Axelar governance module as its governance address.
- Review the prepared proposal file before submission. It should contain only `MsgExecuteContract` messages for
  `MultisigProver` contracts that currently have the source admin.

## Prepare Proposal

Prepare a reviewed contract list containing only chain-specific `MultisigProver` contracts whose live admin is
`axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj`.

The reviewed contract list should have this shape:

```json
[
  {
    "chain": "<chain-name>",
    "address": "<multisig-prover-address>",
    "current_admin": "axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj"
  }
]
```

Build `multisig-prover-admin-transfer.json` from that reviewed list. The proposal should be an expedited governance
proposal with one `/cosmwasm.wasm.v1.MsgExecuteContract` message per reviewed contract. Each message must:

- use the Axelar governance module as `sender`: `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`
- use the reviewed `MultisigProver` address as `contract`
- execute `update_admin` with `new_admin_address` set to `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly`
- include no funds

Example proposal messages:

```json
[
  {
    "@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
    "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
    "contract": "axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0",
    "msg": {
      "update_admin": {
        "new_admin_address": "axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly"
      }
    },
    "funds": []
  },
  {
    "@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
    "sender": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
    "contract": "axelar1v8jrupu2rqpskwgtr69max0ajul92q8z5mdxd505m2hu3xc5jzcqm8zyc6",
    "msg": {
      "update_admin": {
        "new_admin_address": "axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly"
      }
    },
    "funds": []
  }
]
```

Review the proposal artifact:

```bash
jq -r '.[] | [.chain, .address, .current_admin] | @tsv' multisig-prover-admin-transfer-contracts.json
jq -r '.messages[] | [.contract, (.msg.update_admin.new_admin_address)] | @tsv' multisig-prover-admin-transfer.json
jq '.messages | length' multisig-prover-admin-transfer.json
```

## Submit Proposal

```bash
axelard tx gov submit-proposal multisig-prover-admin-transfer.json \
  --from <proposer-key> \
  --chain-id axelar-dojo-1 \
  --node "$AXELAR_RPC" \
  --gas auto \
  --gas-adjustment 1.4 \
  -y
```

Vote and wait for the proposal to pass.

## Verify

After the proposal executes, verify every contract included in `multisig-prover-admin-transfer.json` has
`axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` as its admin.

The release is complete when every included contract returns the new admin address.
