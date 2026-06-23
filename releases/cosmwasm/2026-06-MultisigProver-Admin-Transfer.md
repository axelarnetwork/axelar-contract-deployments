# MultisigProver Admin Transfer

|                | **Owner** |
| -------------- | --------- |
| **Created By** | rista404  |
| **Deployment** | rista404  |

| **Network** | **Deployment Status** | **Date**   |
| ----------- | --------------------- | ---------- |
| **Mainnet** | Deployed              | 2026-06-17 |

## Background

Transfer the `adminAddress` for every chain-specific `MultisigProver` contract whose current admin was
`axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj`.

This transfer had to be executed through Axelar governance. `MultisigProver` supports `update_admin`, but the message
is guarded by the `Governance` permission, not the `Admin` permission. The previous admin EOA could not rotate this
role directly.

## Scope

- Environment: `mainnet`
- Source admin: `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj`
- Target admin: `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly`
- Contract set: all configured chain-specific `MultisigProver` instances whose live admin matched the source admin

## Governance Proposal

- Proposal: [481](https://axelarscan.io/proposals/481)
- Status: `PROPOSAL_STATUS_PASSED`
- Title: `Rotate MultisigProver Admin for flow, sui, stellar, xrpl-evm, plume, hedera, berachain, hyperliquid, monad.`
- Submitted: `2026-06-16T12:44:32Z`
- Voting ended: `2026-06-17T12:44:32Z`
- Expedited: `true`

Each proposal message was a `/cosmwasm.wasm.v1.MsgExecuteContract` sent by the Axelar governance module:
`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`.

Each message executed:

```json
{
  "update_admin": {
    "new_admin_address": "axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly"
  }
}
```

## Updated Contracts

| Chain         | MultisigProver Contract                                                        |
| ------------- | ------------------------------------------------------------------------------ |
| `flow`        | `axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0`           |
| `sui`         | `axelar1v8jrupu2rqpskwgtr69max0ajul92q8z5mdxd505m2hu3xc5jzcqm8zyc6`           |
| `stellar`     | `axelar1wdgp5xyqjyv5zsq86n6pah2lsmd46mn0gt4055mvvk6mezn9skqs6p93dg`           |
| `xrpl-evm`    | `axelar198xehj5htckk75s8wcamxerxtdc45669zdqjmr69guveqntj9f6s5rqq55`           |
| `plume`       | `axelar1ll4yhqtldlgqwqthyffqln3cyr2f8ydzhv0djpjyp6sk4v5k4kqqrs60s7`           |
| `hedera`      | `axelar1e7z2faehrvpwl3apq3srr8djp386urvm2fgw3yafmju6slphhe8skecrwk`           |
| `berachain`   | `axelar1k483q898t5w0acqzxhdjlsmnpgcxxa49ye8m46757n8mtk70ugtsu927xw`           |
| `hyperliquid` | `axelar1fxd8rq5j6wluyc07vl9vqr4xmdxxm25l2gd6m2an20mn5fdnzy6qll2nxx`           |
| `monad`       | `axelar1dt6apz0m2lkuls3ah2h7zw277r0v50668fxytqtdxv83yzs4n69qlutnpk`           |

`solana` was not included because its `MultisigProver` admin was already
`axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly`.

## Verification

After proposal execution, all nine included contracts were verified to have:

```text
adminAddress = axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly
```

The release is complete.
