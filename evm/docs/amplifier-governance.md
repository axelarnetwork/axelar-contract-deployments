## Amplifier governance proposals (no relayers / manual proof)

When submitting **Amplifier-style governance proposals** (i.e. a Cosmos “AxelarnetGateway `call_contract`” that targets an EVM chain), there may be **no relayers** configured. In that case you must:

- submit the proposal on the Amplifier chain (Axelar)
- wait for it to pass
- manually **construct proof** on the destination chain’s `MultisigProver` (CosmWasm)
- manually **submit proof** to the destination chain’s EVM `AxelarGateway`
- extract the `commandId` from the `MessageApproved` event
- finally **execute the governance call** on the destination chain’s governance contract using that `commandId`

### Prerequisites

- `axelard` installed and configured for the right environment / keyring.
- A funded EVM key for the destination chain(s): `PRIVATE_KEY`.
- A funded Axelar key (mnemonic) for proposal submission (unless you use `--generate-only`): `MNEMONIC`.

### 1) Create + submit the proposal (Amplifier -> destination EVM chain)

Run `evm/governance.js` over the **destination chain** you want to affect (not `axelar`).

Example: schedule an **operator proposal** to upgrade a contract:

```bash
export ENV=devnet-amplifier
export CHAINS=avalanche-fuji
export PRIVATE_KEY=...
export MNEMONIC="..."

# Choose an activation time in UTC: YYYY-MM-DDTHH:mm:ss
export ACTIVATION_TIME="2025-12-31T00:00:00"
export NEW_IMPL="0xdB7d6A5B8d37a4f34BC1e7ce0d0B8a9DDA124871"

ts-node evm/governance.js schedule-operator upgrade "$ACTIVATION_TIME" \
  -c AxelarServiceGovernance \
  --targetContractName InterchainTokenFactory \
  --implementation "$NEW_IMPL"
```

Notes:
- Pass `--standardProposal` if you want a standard (non-expedited) Amplifier proposal. Default is expedited.
- Use `--generate-only <file.json>` if you want to **generate** the Axelar proposal JSON instead of submitting it.

### 2) Wait for proposal pass + locate the routed message

Find the passed proposal on the Amplifier explorer, then find the transaction that routed the message.

- Example proposal page: `https://devnet-amplifier.axelarscan.io/proposals/1534`
- Blocks view: `https://devnet-amplifier.axelarscan.io/blocks`
- Example end-block with `proposal_passed`: `https://devnet-amplifier.axelarscan.io/block/11758516`

From the tx that contains the `wasm-message routed` event, record:

- `MESSAGE_ID`
- `SRC_CHAIN`

### 3) Construct proof on destination chain’s `MultisigProver` (CosmWasm)

Set variables (get `DST_MULTISIG_PROVER` from config: `axelar.contracts.MultisigProver["$DST_CHAIN"].address`):

```bash
SRC_CHAIN=axelar
MESSAGE_ID=...
DST_MULTISIG_PROVER=0x...
```

Create the `cc_id` JSON used by the prover:

```bash
export CC_ID="{\"source_chain\":\"$SRC_CHAIN\",\"message_id\":\"$MESSAGE_ID\"}"
```

Construct proof:

```bash
echo "$KEYRING_PASSWORD" | axelard tx wasm execute "$DST_MULTISIG_PROVER" \
  "{\"construct_proof\":[${CC_ID}]}" \
  --from validator \
  --gas auto --gas-adjustment 3 \
  -y
```

Open the tx on explorer and search for `multisig_session_id` from events, and save it

- Example tx: `https://devnet-amplifier.axelarscan.io/tx/256A48A63A860F0C2C2B4AB192E5CA891FE84A01DDE413A5DF04CE872A435353` # skip-check

```
MULTISIG_SESSION_ID=...
```

### 4) Poll the proof until it’s completed (optional)

```bash
export MULTISIG_SESSION_ID=...

echo "$KEYRING_PASSWORD" | axelard q wasm contract-state smart "$DST_MULTISIG_PROVER" \
  "{\"proof\":{\"multisig_session_id\":\"$MULTISIG_SESSION_ID\"}}"
```

Wait until you see `status.completed.execute_data` in the response.

### 5) Submit proof to the destination EVM `AxelarGateway`

This uses the destination chain’s MultisigProver session to submit the gateway `execute_data` on EVM:

```bash
ts-node evm/gateway.js \
  --action submitProof \
  --multisigSessionId "$MULTISIG_SESSION_ID"
```

On the destination chain explorer, find the `MessageApproved` event and copy its `commandId` (located at `topics[1]`).

- Example tx: `https://testnet.snowtrace.io/tx/0xb4557afc7690d01e5c0d1062da2ff068a38d8e07bceedef41478cee085518cf6` # skip-check
- Example `commandId`: `0xe12b6dccdaea04f9a1a129b7d07dcad22c5ed8b33c0344e17b34eb77d92e67a5`

### 6) Execute the governance call on the destination chain (using `commandId`)

Important: you must pass the **same action + parameters** you used when creating the proposal in step 1, otherwise the payload won’t match the approved `payload_hash`.

Note: `submit` / `submit-operator` submit the underlying GMP message to the destination governance contract (they call `governance.execute(...)`). They do **not** execute the final target call directly. After eligibility/approval, execute the proposal with `execute` or `execute-operator-proposal`.

For **operator proposals**:

```bash
export COMMAND_ID=0xe12b6dccdaea04f9a1a129b7d07dcad22c5ed8b33c0344e17b34eb77d92e67a5

ts-node evm/governance.js submit-operator schedule-operator upgrade "$COMMAND_ID" "$ACTIVATION_TIME" \
  -c AxelarServiceGovernance \
  --targetContractName InterchainTokenFactory \
  --implementation "$NEW_IMPL"
```

If you are **cancelling** an operator proposal (and relayers don’t execute the GMP), submit the cancellation GMP with `submit-operator cancel-operator`:

```bash
ts-node evm/governance.js submit-operator cancel-operator upgrade "$COMMAND_ID" "$ACTIVATION_TIME" \
  -c AxelarServiceGovernance \
  --targetContractName InterchainTokenFactory \
  --implementation "$NEW_IMPL"
```

For **timelock proposals** (non-operator), use `submit` instead:

```bash
ts-node evm/governance.js submit schedule upgrade "$COMMAND_ID" "$ACTIVATION_TIME" \
  -c AxelarServiceGovernance \
  --targetContractName AxelarGateway \
  --implementation "$NEW_IMPL"
```

If you are **cancelling** a timelock proposal (and relayers don’t execute the GMP), submit the cancellation GMP with `submit cancel`:

```bash
ts-node evm/governance.js submit cancel upgrade "$COMMAND_ID" "$ACTIVATION_TIME" \
  -c AxelarServiceGovernance \
  --targetContractName AxelarGateway \
  --implementation "$NEW_IMPL"
```

### 7) Execute the proposal itself (after ETA / eligibility)

Once the proposal is eligible on the destination chain, execute it:

- timelock: `ts-node evm/governance.js execute --proposal <payload>`
- operator proposal: `ts-node evm/governance.js execute-operator-proposal --proposal <payload>`

You can get `<payload>` from:
- the output printed by step 1 (it prints the proposal payload), or
- your `--generate-only <file>` output (look for `payload` inside the generated JSON).
