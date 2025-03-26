# Update Celo confirmation height

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

## Param change instructions

Create a `proposal.json` file as follows:

```json
{
    "title": "Update Celo confirmation height",
    "description": "Celo supports finalized tag now, therefore updating confirmation height to 2500",
    "changes": [
        {
            "subspace": "evm_celo",
            "key": "confirmationHeight",
            "value": "2500"
        }
    ],
    "deposit": "2000000000uaxl"
}
```

```jsx
axelard tx gov submit-proposal param-change proposal.json --from validator --gas auto --gas-adjustment 1.2
```

Proposal id will be in the output, but can also be seen on the explorer.

Post in the appropriate validator announcement channel about the proposal. Vote with internal validators on `testnet`, `stagenet` and `devnet-amplifier`

```jsx
axelard tx gov vote <proposal_id> yes --from validator
```
