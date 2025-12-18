## Pause / Unpause and Pauser Roles

### Overview

This guide covers **pause/unpause controls** and related **pauser roles** for:

- **Interchain Token Service (ITS)**: global pause status on a chain.
- **AxelarTransceiver**: pauser capability for transceiver contracts used with the gateway.

These are key access-control levers used during incident response and operational handovers.

All examples assume:

```bash
ts-node evm/<script> <command> [args] -e <env> -n <chain>
```

where:
- `-e, --env` is the deployment environment (e.g. `mainnet`, `testnet`).
- `-n, --chain` is the EVM chain name from the Axelar chains config.

---

## Interchain Token Service Pause Status

### What It Does

The ITS contract supports a **global pause flag** on each chain. When paused, ITS operations on that chain are restricted according to the contract’s implementation.  
Pause/unpause is controlled by the **ITS owner or operator** via `evm/its.js`.

### Command: Set Pause Status

```bash
ts-node evm/its.js set-pause-status <pause-status> -e <env> -n <chain>
```

- **`pause-status`**: `true` to pause, `false` to unpause.

**Requirements:**
- Caller must be the **owner or operator** of the ITS contract on the target chain.
- The command uses `setPauseStatus(bool)` and emits `Paused` / `Unpaused` events.

#### Examples

**Pause ITS on a chain:**

```bash
ts-node evm/its.js set-pause-status true -e mainnet -n ethereum --privateKey 0x...
```

**Unpause ITS on a chain:**

```bash
ts-node evm/its.js set-pause-status false -e mainnet -n ethereum --privateKey 0x...
```

### Recommended Workflow

**Emergency response:**
1. **Pause ITS** on the affected chain:
   ```bash
   ts-node evm/its.js set-pause-status true -e mainnet -n ethereum --privateKey 0x...
   ```
2. Optionally **freeze high-risk tokens** using flow limits (see `rate-limits.md`).
3. Investigate and coordinate with stakeholders.
4. **Unpause ITS** once the issue is resolved:
   ```bash
   ts-node evm/its.js set-pause-status false -e mainnet -n ethereum --privateKey 0x...
   ```

---

## AxelarTransceiver Pauser Capability

### What It Does

`AxelarTransceiver` contracts have a **pauser** role that can:
- Pause/unpause specific cross‑chain messaging paths in the associated protocol.
- Act as a targeted control separate from ITS’ global pause.

The deployment scripts in `evm/axelar-transceiver.ts` support:
- Reading the current `pauser` and `owner`.
- **Transferring the pauser capability** to a new address.

### Prerequisites

- AxelarTransceiver is deployed and configured (see main `README.md`).
- You have:
  - The **artifact path** for the transceiver build.
  - The correct `--transceiverPrefix` for the contract (e.g. `Lido`, `Monad`).
  - A wallet with sufficient permissions to transfer the pauser role.

---

### Command: Transfer Pauser Capability

```bash
ts-node evm/axelar-transceiver.ts transfer-pauser <pauser-address> \
  --artifactPath <path-to-artifacts> \
  --transceiverPrefix <TRANSCEIVER_PREFIX> \
  -e <env> -n <chain>
```

- **`pauser-address`**: New account that will receive the pauser role.
- **`--artifactPath`**: Path to the prebuilt AxelarTransceiver artifacts.
- **`--transceiverPrefix`**: Prefix used in the chains config for this transceiver.


#### Example

```bash
ts-node evm/axelar-transceiver.ts transfer-pauser 0xNewPauserAddress \
  --artifactPath /path/to/example-wormhole-axelar-wsteth/out/ \
  --transceiverPrefix Lido \
  -e mainnet -n ethereum
```

---


## Troubleshooting

| Error / Symptom | Likely Cause | Suggested Fix |
|-----------------|-------------|---------------|
| `set-pause-status` revert / “can only be performed by contract owner or operator” | Caller is not ITS owner or operator. | Use the ITS owner or operator wallet for the target chain. |
| `transfer-pauser` revert / unauthorized | Caller does not have permission on the transceiver. | Use the transceiver owner/authorized account. |
| Unexpected behavior after pausing | Downstream apps are not handling paused state as expected. | Coordinate with application teams to ensure they honor pause semantics and monitor relevant events. |
