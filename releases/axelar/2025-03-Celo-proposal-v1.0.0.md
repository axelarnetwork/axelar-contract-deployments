# Update Celo confirmation height

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io> |

| **Network**          | **Deployment Status** |  **Date**  |
| -------------------- | --------------------- | --------   |
| **Devnet Amplifier** | NA                    | -          |
| **Stagenet**         | NA                    | -          |
| **Testnet**          | Done                  | 2025-03-27 |
| **Mainnet**          | In Progress           | 2025-03-26 |

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
axelard tx gov vote <proposal_id> [yes|no] --from validator
```

## Checklist

The following checks should be performed after the rollout

1. Check if proposal is up from: 

| **Network**          | Explorer |
| -------------------- | --------------------- |
| **Testnet**          | https://testnet.axelarscan.io/proposal/${proposal-id} |
| **Mainnet**          | https://axelarscan.io/proposal/${proposal-id}         |

2. Check if proposal can be passed: 

| **Network**          | Explorer |
| -------------------- | --------------------- |
| **Testnet**          | https://www.mintscan.io/axelar-testnet/proposals/${proposal-id} |
| **Mainnet**          | https://www.mintscan.io/axelar/proposals/${proposal-id}         |

Can also be checked from

```jsx
axelard query gov proposal [proposal-id]
```

3. Check if the param was updated on the network

```jsx
axelard query evm params celo
```

4. Check if GMP calls are being executed from: https://axelarscan.io/gmp/search?sourceChain=celo

