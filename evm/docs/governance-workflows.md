# Governance Workflows

To learn more about how Governance works, first checkout [governance guide](governance.md)

## End to End Workflows

### Upgrade Workflow

This section provides a complete step-by-step guide for upgrading contracts (AxelarGateway, InterchainTokenService, or AxelarGasService) using interchain governance.

1. Schedule the Upgrade Proposal

Schedule the upgrade proposal with a future execution date (ETA). The ETA must be at least `minimumTimeLockDelay` seconds in the future.

```bash
# Upgrade AxelarGateway
ts-node evm/governance.js schedule upgrade <activationTime> \
  --targetContractName AxelarGateway \
  --implementation 0xNewImplementationAddress

# Upgrade InterchainTokenService
ts-node evm/governance.js schedule upgrade <activationTime> \
  --targetContractName InterchainTokenService \
  --implementation 0xNewImplementationAddress

# Upgrade AxelarGasService
ts-node evm/governance.js schedule upgrade <activationTime> \
  --targetContractName AxelarGasService \
  --implementation 0xNewImplementationAddress
```

**Note:** If `--implementation` is not provided, the script will use the implementation address from the chain configuration **only for `AxelarGateway`**. For other contracts (InterchainTokenService, AxelarGasService), you **must** provide the `--implementation` address.

2. Submit to Axelar

The proposal will be submitted automatically if `MNEMONIC` is set in `.env`, or submit via Cosmos CLI as shown in the "Submission Methods" section in `governance.md`.

3. Wait for Voting Period

Wait for validators and the community to vote on the proposal. Monitor the proposal status on Axelarscan or via:

```bash
axelard q gov proposal <proposal-id> --node <rpc>
```

4. Check if GMP Call Was Executed

After the proposal passes, a GMP call should be automatically executed by relayers. Check Axelarscan:

1. Go to https://axelarscan.io/gmp/search
2. Filter by:
   - **Source Chain**: `axelar`
   - **Method**: `Call Contract`
   - **Destination Chain**: Your target chain
3. Look for the most recent GMP call matching your proposal

5. Manual Submission

If relayers didn't execute the GMP call automatically, manually submit it:

```bash
# Get the commandId from Axelarscan (see Step 4)
ts-node evm/governance.js submit upgrade <commandId> <activationTime> \
  --targetContractName AxelarGateway \
  --implementation 0xNewImplementationAddress
```

6. Check Proposal ETA and Verify It Has Passed

After the proposal is scheduled on the EVM chain, check when it can be executed. You need to provide the same `target` and `calldata` that were used when scheduling the proposal:

```bash
# Using target and calldata directly
ts-node evm/governance.js eta --target 0xGatewayAddress --calldata 0xCalldataHex

# OR using --proposal (if you have the encoded governance proposal payload)
# The proposal payload is the encoded governance command: encode(commandType, target, calldata, nativeValue, eta)
ts-node evm/governance.js eta --proposal 0xEncodedProposalPayload
```

**Note:** The `eta` command requires explicit `--target` and `--calldata` (or `--proposal`). It does not support `--targetContractName` because it needs the exact calldata to compute the proposal hash.

**Important:** If the current time is less than the ETA, wait until the ETA passes. The script will show a warning if you try to execute too early.

7. Execute the Proposal

Once the ETA has passed, execute the proposal. You need to provide the same `target` and `calldata` that were used when scheduling:

```bash
# Using target and calldata directly
ts-node evm/governance.js execute --target 0xGatewayAddress --calldata 0xCalldataHex

# OR using --proposal (if you have the encoded proposal payload)
ts-node evm/governance.js execute --proposal 0xEncodedProposalPayload
```

**Note:** The `execute` command requires explicit `--target` and `--calldata` (or `--proposal`). It does not support `--targetContractName` because it needs the exact calldata to compute the proposal hash.

8. Cancel Proposal (if needed)

If you need to cancel a scheduled proposal before execution:

```bash
# Cancel before ETA passes
ts-node evm/governance.js cancel upgrade \
  --targetContractName AxelarGateway \
  --implementation 0xNewImplementationAddress
```

**Note:** You can only cancel proposals that haven't reached their ETA yet.

---

### Raw Workflow

The `raw` action allows you to execute any function on any contract through governance. This section explains how to generate calldata for common raw commands.

1. **Generate Calldata:**

   ```bash
   CALLDATA=$(
     node -e "
       const { utils: { Interface } } = require('ethers');
       const iface = new Interface(['function yourFunctionName(type1,type2)']);
       console.log(iface.encodeFunctionData('yourFunctionName', [param1, param2]));
     "
   )
   echo "Calldata: $CALLDATA"
   ```

2. **Schedule Proposal:**

   ```bash
   ts-node evm/governance.js schedule raw <activationTime> \
     --target <contractAddress> \
     --calldata <generatedCalldata>
   ```

3. **Submit to Axelar** (automatic if `MNEMONIC` is set, or via Cosmos CLI)

4. **Wait for Voting & GMP Execution**

5. **Manual Submit (if relayers failed):**

   ```bash
   ts-node evm/governance.js submit raw <commandId> <activationTime> \
     --target <contractAddress> \
     --calldata <generatedCalldata>
   ```

6. **Check ETA:**

   ```bash
   ts-node evm/governance.js eta --target <contractAddress> --calldata <generatedCalldata>
   ```

7. **Execute after ETA:**

   ```bash
   ts-node evm/governance.js execute --target <contractAddress> --calldata <generatedCalldata>
   ```

---

## Contract-Specific Workflows

### InterchainTokenService Governance Actions

#### Set Trusted Chains via ITS

```bash
ts-node evm/its.js set-trusted-chains ethereum avalanche \
  --governance \
  --activationTime <activationTime> # UTC timestamp or relative seconds
```

#### Remove Trusted Chains via ITS

```bash
ts-node evm/its.js remove-trusted-chains ethereum avalanche \
  --governance \
  --activationTime <activationTime> # UTC timestamp or relative seconds
```

#### Pause / Unpause ITS

```bash
# Pause ITS
ts-node evm/its.js set-pause-status true \
  --governance \
  --activationTime <activationTime> # UTC timestamp or relative seconds
```

```bash
# Unpause ITS
ts-node evm/its.js set-pause-status false \
  --governance \
  --activationTime <activationTime> # UTC timestamp or relative seconds
```

#### Migrate Interchain Token via ITS

```bash
ts-node evm/its.js migrate-interchain-token 0x0000...0000 \
  --governance \
  --activationTime <activationTime> # UTC timestamp or relative seconds
```

### Gateway Governance Actions

#### Transfer Gateway Governance via Governance

```bash
ts-node evm/gateway.js \
  --action transferGovernance \
  --destination 0xNewGovernorAddress \
  --governance \
  --activationTime <activationTime> # UTC timestamp or relative seconds
```

#### Transfer Gateway Operatorship via Governance (Amplifier)

```bash
ts-node evm/gateway.js \
  --action transferOperatorship \
  --newOperator 0xNewOperatorAddress \
  --governance \
  --activationTime <activationTime> # UTC timestamp or relative seconds
```

- Only supported when `AxelarGateway` is configured with `connectionType: "amplifier"`.
- Caller must be either the current operator or the owner on the destination chain.

### Steps After Scheduling a Proposal

Once a proposal has been scheduled on the destination chain, follow these steps:

1. **Wait for Axelar proposal to pass and GMP to be relayed**
   - Monitor the Axelar governance proposal on-chain or via Axelarscan until it reaches `Passed`.
   - Check Axelarscan GMP view (`Source Chain: axelar`, `Method: Call Contract`) to confirm that the GMP call to the destination chain was executed.

2. **If relayers failed, manually submit the GMP call (optional)**
   - Use the `submit` command from `evm/governance.js`:

   ```bash
   ts-node evm/governance.js submit raw <commandId> <activationTime> [options]
   ```

   - **Where:**
     - `<commandId>` is the GMP `commandId` from Axelarscan (see the Submit Proposal section in `governance.md` for how to find it).
     - `<activationTime>` is the same activation time you used when scheduling (UTC timestamp or relative seconds).

3. **Inspect ETA on the destination chain**
   - Once the GMP has executed and the timelock is created on the destination chain, compute ETA with:

   ```bash
   ts-node evm/governance.js eta \
     --target <target> \
     --calldata <calldata>
   ```

   - Use the same `target` and `calldata` that were used when scheduling.

4. **Execute after ETA has passed**

   ```bash
   ts-node evm/governance.js execute \
     --target <target> \
     --calldata <calldata>
   ```

   - Use the same `target` and `calldata` that were used when scheduling.
