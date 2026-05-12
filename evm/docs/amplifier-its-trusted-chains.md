## Setting ITS trusted chains on amplifier EVM chains (manual relay)

This is a step-by-step walkthrough of how to set trusted chains on an **amplifier** EVM chain's
InterchainTokenService contract via governance, and then manually relay the message when no
relayers are configured.

**Context:** On devnet-amplifier, EVM chains like `avalanche-fuji` have `connectionType: "amplifier"`.
The standard `CallContractsProposal` (nexus module) path does **not** work for these chains because
the nexus module only knows about consensus chains. Instead, the proposal must go through
`AxelarnetGateway.call_contract`, and the message must be manually relayed.

### Prerequisites

```bash
# .env file must contain:
ENV=devnet-amplifier
MNEMONIC="<your axelar mnemonic>"      # funded Axelar account for CosmWasm txs
PRIVATE_KEY=0x...                       # funded EVM key for the destination chain
```

Install Node.js dependencies (the repo should already have them):

```bash
npm install
```

---

### Step 1: Submit the governance proposal

Run `evm/its.js set-trusted-chains` with `--governance`. This submits an Axelar governance
proposal via `AxelarnetGateway.call_contract` (the amplifier path).

```bash
ts-node evm/its.js set-trusted-chains solana-18 hub \
  -n avalanche-fuji \
  --governance \
  --env devnet-amplifier \
  --yes
```

**What this does:**
- Encodes a multicall on the ITS contract: `setTrustedAddress("solana-18", "hub")` + `setTrustedAddress("hub", "hub")`
- Wraps it in a governance payload (ScheduleTimelock, target=ITS, eta=0)
- Submits an Axelar governance proposal containing a `MsgExecuteContract` to the AxelarnetGateway
  with a `call_contract` message targeting the destination chain's governance contract

**Output to save:** the proposal number (e.g., `Proposal submitted: 1586`).

---

### Step 2: Wait for the proposal to pass

Governance proposals on devnet-amplifier have a 5-minute voting period and pass automatically
(validators vote yes). Check the proposal status:

```bash
curl -s "http://devnet-amplifier.axelar.dev:1317/cosmos/gov/v1/proposals/<PROPOSAL_ID>" \
  | python3 -c "import json,sys; p=json.load(sys.stdin)['proposal']; print('Status:', p['status'])"
```

Wait until `PROPOSAL_STATUS_PASSED`.

---

### Step 3: Find the routed message ID

Once the proposal passes, the AxelarnetGateway routes the message. Query the gateway to
confirm the message exists and get the message ID.

On devnet-amplifier, the AxelarnetGateway address is in the config at
`axelar.contracts.AxelarnetGateway.address`. The message ID from the proposal execution
follows the format `<block_data_hash>-<event_index>`.

You can verify the message is routable:

```bash
# Set variables from the config
AXELARNET_GW="axelar1wvms3cy5hxrgl7uxhkz7yth4qzqum6aaccwkmvafq8z0mgdfxr8qrnvw0k"
MESSAGE_ID="0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855-588335"

QUERY="{\"routable_messages\":{\"cc_ids\":[{\"source_chain\":\"axelar\",\"message_id\":\"$MESSAGE_ID\"}]}}"
ENCODED=$(echo -n "$QUERY" | base64 -w0)

curl -s "http://devnet-amplifier.axelar.dev:1317/cosmwasm/wasm/v1/contract/$AXELARNET_GW/smart/$ENCODED"
```

Confirm you see the message with `destination_chain`, `destination_address`, and `payload_hash`.

---

### Step 4: Construct proof on the MultisigProver

The MultisigProver is a CosmWasm contract on the Axelar chain that coordinates verifier
signatures to create a cryptographic proof that the EVM gateway will accept.

Get the MultisigProver address from the config:
`axelar.contracts.MultisigProver["<destination-chain>"].address`

Since `axelard` may not be available, use this Node.js script:

```javascript
// construct-proof.js
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { GasPrice } = require('@cosmjs/stargate');
const fs = require('fs');
const path = require('path');
require('dotenv').config({ path: path.join(__dirname, '.env') });

const CONFIG_PATH = path.join(__dirname, 'axelar-chains-config/info/devnet-amplifier.json');

// ── CONFIGURE THESE ──
const MESSAGE_ID = '0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855-588335';
const SOURCE_CHAIN = 'axelar';
const DST_CHAIN = 'avalanche-fuji';
// ─────────────────────

async function main() {
    const config = JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'));
    const axelar = config.axelar;
    const rpc = axelar.rpc;
    const gasPrice = GasPrice.fromString(axelar.gasPrice);
    const mnemonic = process.env.MNEMONIC;

    if (!mnemonic) throw new Error('MNEMONIC env var is required');

    const multisigProverAddress = axelar.contracts.MultisigProver[DST_CHAIN].address;
    console.log('MultisigProver:', multisigProverAddress);
    console.log('Message ID:', MESSAGE_ID);

    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
    const accounts = await wallet.getAccounts();
    console.log('Sender:', accounts[0].address);

    const client = await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice });

    const msg = {
        construct_proof: [{
            source_chain: SOURCE_CHAIN,
            message_id: MESSAGE_ID,
        }],
    };

    console.log('\nExecuting construct_proof...');
    const tx = await client.execute(accounts[0].address, multisigProverAddress, msg, 'auto', '');

    console.log('TX hash:', tx.transactionHash);

    // Extract multisig_session_id
    const proofEvent = tx.events.find(e =>
        e.type === 'wasm-proof_under_construction' || e.type === 'wasm-signing_started',
    );

    if (proofEvent) {
        const sessionIdAttr = proofEvent.attributes.find(a => a.key === 'multisig_session_id');
        if (sessionIdAttr) {
            console.log('\n=== MULTISIG_SESSION_ID:', sessionIdAttr.value, '===');
        }
    } else {
        console.log('\nAll events:');
        for (const ev of tx.events) {
            console.log(`  ${ev.type}:`, ev.attributes.map(a => `${a.key}=${a.value}`).join(', '));
        }
    }
}

main().catch(console.error);
```

Run it:

```bash
node construct-proof.js
```

**Output to save:** the `MULTISIG_SESSION_ID` (e.g., `23318`).

---

### Step 5: Poll the proof until completed

The verifiers sign the proof asynchronously. Poll until `status.completed` appears:

```javascript
// poll-proof.js
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

const MULTISIG_PROVER = 'axelar1p22kz5jr7a9ruu8ypg40smual0uagl64dwvz5xt042vu8fa7l7dsl3wx8q';
const SESSION_ID = '23318'; // from step 4

(async () => {
    const client = await CosmWasmClient.connect('http://devnet-amplifier.axelar.dev:26657');
    const result = await client.queryContractSmart(MULTISIG_PROVER, {
        proof: { multisig_session_id: SESSION_ID },
    });

    const status = result.status;

    if (status.completed) {
        console.log('COMPLETED!');
        console.log('execute_data length:', status.completed.execute_data.length);
    } else {
        console.log('Not yet completed. Status:', Object.keys(status));
        console.log('Try again in a few seconds...');
    }
})().catch(console.error);
```

```bash
node poll-proof.js
```

On devnet-amplifier this typically completes within a few seconds.

---

### Step 6: Submit proof to the destination EVM AxelarGateway

This sends the signed proof to the EVM chain's AxelarGateway contract, which approves the
message and emits a `MessageApproved` event.

```bash
ts-node evm/gateway.js \
  --action submitProof \
  --multisigSessionId 23318 \
  -n avalanche-fuji \
  --env devnet-amplifier \
  --yes
```

**Output:** a broadcasted tx hash (e.g., `0xe3fbf0b4...`).

---

### Step 7: Extract the commandId from the MessageApproved event

Query the EVM tx receipt to get the `commandId`:

```javascript
// extract-command-id.js
const { ethers } = require('ethers');

const TX_HASH = '0xe3fbf0b4a316b8b3a67a8d9493180cfb32bbbb04ce8932992864a80e28edad18'; // from step 6
const RPC = 'https://api.avax-test.network/ext/bc/C/rpc';

(async () => {
    const provider = new ethers.providers.JsonRpcProvider(RPC);
    const receipt = await provider.getTransactionReceipt(TX_HASH);

    const messageApprovedTopic = ethers.utils.id(
        'MessageApproved(bytes32,string,string,string,address,bytes32)',
    );

    for (const log of receipt.logs) {
        if (log.topics[0] === messageApprovedTopic) {
            console.log('commandId:', log.topics[1]);

            const decoded = new ethers.utils.AbiCoder().decode(
                ['string', 'string', 'string'],
                log.data,
            );
            console.log('sourceChain:', decoded[0]);
            console.log('messageId:', decoded[1]);
            console.log('sourceAddress:', decoded[2]);
        }
    }
})().catch(console.error);
```

```bash
node extract-command-id.js
```

**Output to save:** the `commandId` (e.g., `0xe814c00d56ac165ff2942c156d0bdb1d26a544b6c7859a1ff80692c9875da860`).

---

### Step 8: Execute the GMP message on the governance contract

Call `governance.execute(commandId, sourceChain, sourceAddress, payload)` on the destination
chain's governance contract. This delivers the governance proposal and schedules it.

**Important:** The `payload` must be the **exact** bytes that were sent via `call_contract`.
For amplifier chains, `sourceChain` is `"axelar"` (not `"Axelarnet"`).

```javascript
// execute-governance.js
require('dotenv').config();
const { ethers } = require('ethers');
const fs = require('fs');
const path = require('path');

const CONFIG_PATH = path.join(__dirname, 'axelar-chains-config/info/devnet-amplifier.json');

// ── CONFIGURE THESE ──
const COMMAND_ID = '0xe814c00d56ac165ff2942c156d0bdb1d26a544b6c7859a1ff80692c9875da860';
const SOURCE_CHAIN = 'axelar';
const SOURCE_ADDRESS = 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj'; // GOVERNANCE_MODULE_ADDRESS
const DST_CHAIN = 'avalanche-fuji';

// The exact payload from the proposal (hex string, no 0x prefix).
// Copy this from the proposal output: the "payload" field in the amplifier-chain proposal JSON.
const PAYLOAD_HEX = '00000000000000000000000000000000000000000000000000000000000000000000000000000000000000002269b93c8d8d4afce9786d2940f5fcd4386db7ff00000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000284ac9650d8000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000000c49f409d77000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000009736f6c616e612d31380000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000368756200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c49f409d770000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000036875620000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000368756200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000';
// ─────────────────────

const GOVERNANCE_ABI = [
    'function execute(bytes32 commandId, string calldata sourceChain, string calldata sourceAddress, bytes calldata payload) external',
    'event ProposalScheduled(bytes32 indexed proposalHash, address indexed target, bytes calldata, uint256 value, uint256 eta)',
];

async function main() {
    const config = JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'));
    const chain = config.chains[DST_CHAIN];
    const governanceAddress = chain.contracts.AxelarServiceGovernance.address;

    console.log('Governance contract:', governanceAddress);
    console.log('Command ID:', COMMAND_ID);

    const provider = new ethers.providers.JsonRpcProvider(chain.rpc);
    const wallet = new ethers.Wallet(process.env.PRIVATE_KEY, provider);
    console.log('Wallet:', wallet.address);

    const governance = new ethers.Contract(governanceAddress, GOVERNANCE_ABI, wallet);

    console.log('\nCalling governance.execute()...');
    const tx = await governance.execute(
        COMMAND_ID,
        SOURCE_CHAIN,
        SOURCE_ADDRESS,
        '0x' + PAYLOAD_HEX,
        { gasLimit: 500000 },
    );

    console.log('TX hash:', tx.hash);
    const receipt = await tx.wait();
    console.log('Status:', receipt.status === 1 ? 'SUCCESS' : 'FAILED');

    for (const log of receipt.logs) {
        const proposalScheduledTopic = ethers.utils.id(
            'ProposalScheduled(bytes32,address,bytes,uint256,uint256)',
        );

        if (log.topics[0] === proposalScheduledTopic) {
            console.log('\nProposalScheduled event:');
            console.log('  proposalHash:', log.topics[1]);
            console.log('  target:', log.topics[2]);
        }
    }
}

main().catch(console.error);
```

```bash
node execute-governance.js
```

**Output:** `ProposalScheduled` event with `proposalHash` and `target`.

---

### Step 9: Execute the scheduled proposal (after ETA)

The governance contract enforces `minimumTimeLockDelay`. On devnet-amplifier this is very short
(~10 minutes). Once the ETA has passed, execute the proposal.

**Important caveat:** The governance contract may store a slightly different calldata than what
you originally encoded (trailing zero bytes can be stripped by Solidity ABI encoding). If
`ts-node evm/governance.js execute` fails with "Proposal does not exist", you must use the
exact stored calldata from the `ProposalScheduled` event.

#### Option A: Use the CLI (if calldata matches)

```bash
ts-node evm/governance.js execute \
  --target 0x2269B93c8D8D4AfcE9786d2940F5Fcd4386Db7ff \
  --calldata 0xac9650d8... \
  -n avalanche-fuji \
  --env devnet-amplifier \
  -c AxelarServiceGovernance \
  --yes
```

#### Option B: Use a script with stored calldata (if Option A fails)

If Option A gives "Proposal does not exist", use this script that reads the exact stored
calldata from the `ProposalScheduled` event:

```javascript
// execute-proposal.js
require('dotenv').config();
const { ethers } = require('ethers');
const fs = require('fs');
const path = require('path');

const CONFIG_PATH = path.join(__dirname, 'axelar-chains-config/info/devnet-amplifier.json');

// ── CONFIGURE THESE ──
const DST_CHAIN = 'avalanche-fuji';
const GOVERNANCE_EXECUTE_TX = '0x70c5f8de7fbb357546c39ddbe560dbb5549ec6b90bf8229c891dabfafd21b33c'; // from step 8
// ─────────────────────

const GOV_ABI = [
    'function getProposalEta(address target, bytes calldata callData, uint256 nativeValue) view returns (uint256)',
    'function executeProposal(address target, bytes calldata callData, uint256 nativeValue) external payable',
    'event ProposalExecuted(bytes32 indexed proposalHash, address indexed target, bytes calldata, uint256 value, uint256 eta)',
];

async function main() {
    const config = JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'));
    const chain = config.chains[DST_CHAIN];
    const governanceAddress = chain.contracts.AxelarServiceGovernance.address;

    const provider = new ethers.providers.JsonRpcProvider(chain.rpc);
    const wallet = new ethers.Wallet(process.env.PRIVATE_KEY, provider);

    // Get the exact stored calldata from the ProposalScheduled event in step 8
    const receipt = await provider.getTransactionReceipt(GOVERNANCE_EXECUTE_TX);
    const govLog = receipt.logs.find(l => l.address.toLowerCase() === governanceAddress.toLowerCase());
    const decoded = new ethers.utils.AbiCoder().decode(['bytes', 'uint256', 'uint256'], govLog.data);

    const storedCalldata = decoded[0];
    const target = '0x' + govLog.topics[2].slice(26); // extract address from indexed topic
    const nativeValue = decoded[1];

    console.log('Target:', target);
    console.log('Stored calldata length:', storedCalldata.length);

    const governance = new ethers.Contract(governanceAddress, GOV_ABI, wallet);

    // Check ETA
    const eta = await governance.getProposalEta(target, storedCalldata, nativeValue);
    const now = Math.floor(Date.now() / 1000);
    console.log('ETA:', new Date(eta.toNumber() * 1000).toISOString());
    console.log('Now:', new Date(now * 1000).toISOString());

    if (eta.eq(0)) {
        console.log('ERROR: Proposal does not exist (already executed or wrong params)');
        return;
    }

    if (now < eta.toNumber()) {
        console.log('NOT YET ELIGIBLE. Wait', eta.toNumber() - now, 'seconds.');
        return;
    }

    console.log('\nExecuting proposal...');
    const tx = await governance.executeProposal(target, storedCalldata, nativeValue, {
        value: nativeValue,
        gasLimit: 500000,
    });

    console.log('TX hash:', tx.hash);
    const execReceipt = await tx.wait();
    console.log('Status:', execReceipt.status === 1 ? 'SUCCESS' : 'FAILED');
}

main().catch(console.error);
```

```bash
node execute-proposal.js
```

---

### Step 10: Verify

Confirm the trusted address is now set on the ITS contract:

```javascript
// verify-trusted.js
const { ethers } = require('ethers');

const RPC = 'https://api.avax-test.network/ext/bc/C/rpc';
const ITS = '0x2269B93c8D8D4AfcE9786d2940F5Fcd4386Db7ff';

(async () => {
    const provider = new ethers.providers.JsonRpcProvider(RPC);
    const its = new ethers.Contract(ITS, [
        'function trustedAddress(string calldata chain) view returns (string)',
    ], provider);

    console.log('trustedAddress("solana-18"):', await its.trustedAddress('solana-18'));
    console.log('trustedAddress("hub"):', await its.trustedAddress('hub'));
})().catch(console.error);
```

```bash
node verify-trusted.js
```

Expected output:
```
trustedAddress("solana-18"): hub
trustedAddress("hub"): hub
```

---

### Summary of addresses used (devnet-amplifier, avalanche-fuji)

| Contract | Address |
|----------|---------|
| AxelarnetGateway (Axelar chain) | `axelar1wvms3cy5hxrgl7uxhkz7yth4qzqum6aaccwkmvafq8z0mgdfxr8qrnvw0k` |
| MultisigProver for avalanche-fuji (Axelar chain) | `axelar1p22kz5jr7a9ruu8ypg40smual0uagl64dwvz5xt042vu8fa7l7dsl3wx8q` |
| AxelarGateway (avalanche-fuji EVM) | `0xF128c84c3326727c3e155168daAa4C0156B87AD1` |
| AxelarServiceGovernance (avalanche-fuji EVM) | `0xBCd0E8c0E2D12a606c4434Bcd538FE5b451f0a4C` |
| InterchainTokenService (avalanche-fuji EVM) | `0x2269B93c8D8D4AfcE9786d2940F5Fcd4386Db7ff` |
| Governance module (Axelar chain) | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |

### Summary of transactions (proposal 1586)

| Step | TX Hash | Chain |
|------|---------|-------|
| 1. Submit proposal | Proposal 1586 | Axelar |
| 4. construct_proof | `2AA5D379288D3728B2430A99FCD23CE37CEEC47AC122ABD11E26468E053F2EB9` | Axelar |
| 6. submitProof | `0xe3fbf0b4a316b8b3a67a8d9493180cfb32bbbb04ce8932992864a80e28edad18` | avalanche-fuji |
| 8. governance.execute | `0x70c5f8de7fbb357546c39ddbe560dbb5549ec6b90bf8229c891dabfafd21b33c` | avalanche-fuji |
| 9. executeProposal | `0xb48419955242ab4b8039d72c2e3e9307ca1e2ae3a5c280c31da6e5fa412252f4` | avalanche-fuji |

### Key values from this run

| Value | Data |
|-------|------|
| Message ID | `0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855-588335` |
| Multisig session ID | `23318` |
| commandId | `0xe814c00d56ac165ff2942c156d0bdb1d26a544b6c7859a1ff80692c9875da860` |
| Proposal hash | `0x829d7659c7737b08cfde4f40ee39735c242fbcc713433cfb360dd6cdb0693fef` |
