# Contract Ownership Management

The `ownership.js` script manages contracts that implement `IOwnable`. It supports both **direct EOA execution** and **governance proposals** (timelock or operator-based).

## Prerequisites

- Set the environment (`-e` or `--env`) - defaults to `testnet` if not specified. See [README.md](../README.md#setup) for setup instructions.
- For direct EOA execution, provide a private key (`-p` or `--privateKey`) or set `PRIVATE_KEY` environment variable.
- For governance proposal submission, provide a mnemonic (`-m` or `--mnemonic`) or set `MNEMONIC` environment variable.

## Direct actions (EOA)

```bash
# Query current owner
ts-node evm/ownership.js -n <chain> -c AxelarGateway --action owner

# Query pending owner
ts-node evm/ownership.js -n <chain> -c AxelarGateway --action pendingOwner

# Transfer ownership directly
ts-node evm/ownership.js -n <chain> -c AxelarGateway --action transferOwnership --newOwner 0xNewOwnerAddress

# Propose ownership
ts-node evm/ownership.js -n <chain> -c AxelarGateway --action proposeOwnership --newOwner 0xNewOwnerAddress

# Accept ownership
ts-node evm/ownership.js -n <chain> -c AxelarGateway --action acceptOwnership
```

## Governance proposals

### Timelock-based (default)

```bash
# Generate proposal JSON (no mnemonic required)
ts-node evm/ownership.js --governance -n <chain> -c AxelarGateway \
  --action transferOwnership --newOwner 0xNewOwnerAddress \
  --activationTime <YYYY-MM-DDTHH:mm:ss|relative-seconds>

# Generate to file
ts-node evm/ownership.js --governance --generate-only ownership-proposal.json \
  -n <chain> -c AxelarGateway --action transferOwnership --newOwner 0xNewOwnerAddress \
  --activationTime <YYYY-MM-DDTHH:mm:ss|relative-seconds>

# Submit (requires MNEMONIC)
ts-node evm/ownership.js --governance \
  -n <chain> -c AxelarGateway --action transferOwnership --newOwner 0xNewOwnerAddress \
  --activationTime <YYYY-MM-DDTHH:mm:ss|relative-seconds>
```

### Operator-based (bypass timelock)

Use `AxelarServiceGovernance` with `--operatorProposal` to skip timelock via operator approval:

```bash
ts-node evm/ownership.js --governance --operatorProposal \
  -n <chain> -c AxelarGateway --action transferOwnership --newOwner 0xNewOwnerAddress
```

## Governance options

- `--governance`: Enable proposal mode (no on-chain transaction from your EOA)
- `--governanceContract <InterchainGovernance|AxelarServiceGovernance>`: Governance contract to target (default: `AxelarServiceGovernance`)
- `--operatorProposal`: Generate operator-style proposal (bypasses timelock; requires `AxelarServiceGovernance`)
- `--activationTime <time>`: ETA as `YYYY-MM-DDTHH:mm:ss` UTC or relative seconds (e.g., `3600` for 1 hour from now)
- `--generate-only <file>`: Write proposal JSON to file instead of submitting
- `--mnemonic`: Mnemonic for submitting to Axelar (uses `MNEMONIC` environment variable if set)

## Notes

- In proposal mode, calldata is generated via the ABI using `populateTransaction` for safety.
- Multi-chain execution is supported via `-n chainA,chainB` (use `--parallel` for concurrency).
- If `--operatorProposal` is set with `InterchainGovernance`, the command will fail because operator approvals require `AxelarServiceGovernance`.
- Use `--address <address>` to override the contract address from chain configuration.
