## Manual Proof Construction for Stuck Amplifier Messages

When an Amplifier cross-chain message gets stuck (e.g. verifiers don't vote, relayer doesn't pick it up), you can manually construct a proof and submit it to unblock the message.

This guide covers the full flow for messages routed through the Axelar hub to an EVM destination chain (e.g. Hedera).

### Prerequisites

1. **Funded Axelar wallet** — needs a small amount of AXL for gas (~0.1 AXL)
2. **Funded EVM wallet** — for submitting proof to the destination chain gateway
3. **Message details** — from Axelarscan:
    - `SOURCE_CHAIN`: the source chain of the stuck message (often `axelar` for hub-routed messages)
    - `MESSAGE_ID`: the full message ID (e.g. `0xabc...def-123456`)
    - `DST_CHAIN`: the destination chain name as it appears in `testnet.json` / `mainnet.json`

### Step 1: Identify the stuck message

Find the message on Axelarscan. For ITS hub-routed messages, there's a parent message (source → axelar) and a child message (axelar → destination).

```bash
# Check message status via API
curl -s "https://testnet.api.axelarscan.io/gmp/searchGMP" \
  -X POST -H "Content-Type: application/json" \
  -d '{"txHash":"<TX_HASH>","txLogIndex":<LOG_INDEX>}'
```

Look for:

- `simplified_status: "sent"` or `status: "called"` = stuck before verification
- No `confirm`, `approved`, or `executed` fields = needs manual proof

### Step 2: Find the MultisigProver address

```bash
# From the config file
node -e "
const config = require('../axelar-chains-config/info/testnet.json');
console.log(config.axelar.contracts.MultisigProver['<DST_CHAIN>'].address);
"
```

### Step 3: Call `construct_proof` on the MultisigProver

This is a permissionless CosmWasm execute call — anyone can trigger it.

```javascript
// construct-proof.js
const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { GasPrice } = require('@cosmjs/stargate');
const fs = require('fs');

const ENV = 'testnet';
const SOURCE_CHAIN = 'axelar';
const MESSAGE_ID = '<YOUR_MESSAGE_ID>';
const DST_CHAIN = '<YOUR_DST_CHAIN>';

async function main() {
    const config = JSON.parse(fs.readFileSync(`axelar-chains-config/info/${ENV}.json`, 'utf8'));
    const { rpc, gasPrice: gasPriceStr } = config.axelar;
    const multisigProverAddress = config.axelar.contracts.MultisigProver[DST_CHAIN].address;
    const mnemonic = process.env.MNEMONIC;

    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
    const accounts = await wallet.getAccounts();
    const client = await SigningCosmWasmClient.connectWithSigner(rpc, wallet, {
        gasPrice: GasPrice.fromString(gasPriceStr),
    });

    console.log('Sender:', accounts[0].address);
    console.log('MultisigProver:', multisigProverAddress);

    const msg = {
        construct_proof: [
            {
                source_chain: SOURCE_CHAIN,
                message_id: MESSAGE_ID,
            },
        ],
    };

    const tx = await client.execute(accounts[0].address, multisigProverAddress, msg, 'auto', '');
    console.log('TX hash:', tx.transactionHash);

    // Extract multisig_session_id from events
    for (const ev of tx.events) {
        if (ev.type === 'wasm-proof_under_construction' || ev.type === 'wasm-signing_started') {
            const attr = ev.attributes.find((a) => a.key === 'multisig_session_id');
            if (attr) {
                console.log('MULTISIG_SESSION_ID:', attr.value);
                return attr.value;
            }
        }
    }
}

main().catch(console.error);
```

Run it:

```bash
MNEMONIC="<your axelar mnemonic>" node construct-proof.js
```

**Save the `MULTISIG_SESSION_ID`** from the output (e.g. `1907448`).

### Step 4: Poll the proof until completed

Verifiers sign the proof asynchronously. Poll until `status.completed` appears:

```javascript
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

async function poll() {
    const config = require('./axelar-chains-config/info/testnet.json');
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const prover = config.axelar.contracts.MultisigProver['<DST_CHAIN>'].address;

    const result = await client.queryContractSmart(prover, {
        proof: { multisig_session_id: '<SESSION_ID>' }, // plain string, not quoted
    });

    if (result.status.completed) {
        console.log('COMPLETED! execute_data length:', result.status.completed.execute_data.length);
    } else {
        console.log('Status:', Object.keys(result.status)[0]);
    }
}

poll().catch(console.error);
```

Or use the existing query script:

```bash
ts-node cosmwasm/query.ts multisig-proof <DST_CHAIN> <SESSION_ID> -e <ENV>
```

This typically completes within seconds on testnet.

### Step 5: Submit proof to the destination EVM gateway

Once the proof is completed, submit it to the destination chain's AxelarGateway:

```bash
PRIVATE_KEY="<your EVM private key>" ts-node evm/gateway.js \
  --action submitProof \
  --multisigSessionId <SESSION_ID> \
  -n <DST_CHAIN> \
  --env <ENV> \
  -y
```

This calls `gateway.execute(executeData)` which emits a `MessageApproved` event.

### Step 6: Wait for relayer execution (or manually execute)

After `submitProof`, the relayer should automatically call `ITS.execute()` on the destination chain to finalize the token deployment or transfer. This can take a few minutes.

Check status on Axelarscan or directly:

```bash
# Check Axelarscan
curl -s "https://testnet.api.axelarscan.io/gmp/searchGMP" \
  -X POST -H "Content-Type: application/json" \
  -d '{"txHash":"<TX_HASH>","txLogIndex":<LOG_INDEX>}' | python3 -m json.tool
```

If the relayer doesn't pick it up after ~5 minutes, manually call `execute` on the destination ITS contract.

#### Manual ITS execution

First extract the details from the `MessageApproved` event in the `submitProof` TX:

```bash
# Get the submitProof TX receipt
cast receipt <SUBMIT_PROOF_TX> --rpc-url <DST_RPC> --json
```

From the `MessageApproved` event (topic `0xcda53a26...`):

- `topics[1]` = `commandId`
- `topics[2]` = destination contract (ITS address, left-padded)
- `topics[3]` = `payloadHash`
- `data` = ABI-encoded `(string sourceChain, string messageId, string sourceAddress)`

Get the original payload from Axelarscan:

```bash
curl -s "https://testnet.api.axelarscan.io/gmp/searchGMP" \
  -X POST -H "Content-Type: application/json" \
  -d '{"txHash":"<TX_HASH>","txLogIndex":<LOG_INDEX>}' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['data'][0]['call']['returnValues']['payload'])"
```

Then call `execute` on the ITS contract. **Important:** The function signature depends on the gateway version:

- **Legacy gateway** (commandId-based): `execute(bytes32 commandId, string sourceChain, string sourceAddress, bytes payload)`
- **Amplifier gateway** (messageId-based): `execute(string sourceChain, string messageId, string sourceAddress, bytes payload)`

Try `cast call` (static) first to determine which one works before sending:

```bash
# Try commandId-based (legacy) — use this if the static call succeeds
cast call <ITS_ADDRESS> \
  "execute(bytes32,string,string,bytes)" \
  <COMMAND_ID> <SOURCE_CHAIN> <SOURCE_ADDRESS> <PAYLOAD> \
  --rpc-url <DST_RPC>

# If that reverts, try messageId-based (amplifier)
cast call <ITS_ADDRESS> \
  "execute(string,string,string,bytes)" \
  <SOURCE_CHAIN> <MESSAGE_ID> <SOURCE_ADDRESS> <PAYLOAD> \
  --rpc-url <DST_RPC>
```

Then send the transaction:

```bash
cast send <ITS_ADDRESS> \
  "execute(bytes32,string,string,bytes)" \
  <COMMAND_ID> <SOURCE_CHAIN> <SOURCE_ADDRESS> <PAYLOAD> \
  --rpc-url <DST_RPC> \
  --gas-limit 8000000 \
  --private-key <PRIVATE_KEY>
```

The TX logs should show `InterchainTokenDeployed` and `TokenManagerDeployed` events on success.

---

### Concrete example: Hedera testnet (Feb 2026)

**Stuck message:** `0xf5e570dd157fb4aeeba3415bbfa12219b3f45b8be8bbaf65b005cfa97b4d2c4f-335418567`

- Parent: ethereum-sepolia → axelar (deploy aUSDC canonical interchain token to hedera)
- Child: axelar → hedera (the ITS hub forwarded the deployment)
- Stuck at `called` status for days — verifiers never voted

**Resolution:**

| Step               | Detail                                                               |
| ------------------ | -------------------------------------------------------------------- |
| Wallet             | Generated fresh Axelar wallet, funded with 5 AXL from faucet         |
| Address            | `axelar12cgl3zmld540xrz8jes95tcfmjqchgtdeuau27`                      |
| MultisigProver     | `axelar1kleasry5ed73a8u4q6tdeu80hquy4nplfnrntx3n6agm2tcx40fssjk7gj`  |
| construct_proof TX | `4E432C029A596ED53308C0F24D06985886AD0825D5FFB3D8C09F557A456D31F9`   |
| Session ID         | `1907448`                                                            |
| Proof status       | Completed (35 verifier signatures, execute_data: 11912 chars)        |
| submitProof TX     | `0xf9c2508956bcd66be7a2398bd61b8d24d4257d78bc32d275fb2b1ff8975cbf0d` |
| Gateway            | `0xe432150cce91c13a887f7D836923d5597adD8E31` (Hedera testnet)        |
| MessageApproved    | Confirmed in TX logs                                                 |
| Token ID           | `0x7cdcd2fb2a5937353930d06c0b4826bb88d5d0a278c791ec8211824f6efdbe48` |

**Commands used:**

```bash
# 1. construct_proof
MNEMONIC="<mnemonic>" node scripts/construct-proof-hedera.js

# 2. Poll proof (was completed immediately)
# Session ID extracted: 1907448

# 3. Submit proof to Hedera gateway
PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/gateway.js \
  --action submitProof \
  --multisigSessionId 1907448 \
  -n hedera \
  --env testnet \
  -y

# 4. Relayer didn't execute after 5+ min, so manually called ITS.execute
# First tried messageId-style execute(string,string,string,bytes) — reverted
# Then tried commandId-style execute(bytes32,string,string,bytes) — succeeded
cast send 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C \
  "execute(bytes32,string,string,bytes)" \
  0xe7a4b82800d06acfc12805cee8d159715cc8dd2f36881a1edc3079034669f496 \
  "axelar" \
  "axelar1aqcj54lzz0rk22gvqgcn8fr5tx4rzwdv5wv5j9dmnacgefvd7wzsy2j2mr" \
  <PAYLOAD_HEX> \
  --rpc-url "https://testnet.hashio.io/api" \
  --gas-limit 8000000 \
  --private-key $EVM_PRIVATE_KEY
# TX: 0x6d29653a2e273b0791d4262dad64f5a6775e8fb29ce03b37089cfe12c110322b
```

**Result:** aUSDC token deployed on Hedera testnet at `0x000000000000000000000000000000000079AaFB` (Hedera ID: `0.0.7973627`), token manager at `0x4cb6AdFddB8f7B3bA33Ba1D9d8E81d6AE0B9A5C9`, 6 decimals, MINT_BURN type.

### Gotcha: Session ID quoting

The `multisig_session_id` from construct_proof events may come back as a JSON-quoted string (e.g. `"1907448"` instead of `1907448`). When querying the proof status, pass it as a **plain string** — the CosmWasm contract expects a u64, not a quoted string:

```javascript
// Correct
{
    proof: {
        multisig_session_id: '1907448';
    }
}

// Wrong — will fail with "invalid digit found in string"
{
    proof: {
        multisig_session_id: '"1907448"';
    }
}
```

### Related scripts

- `scripts/construct-proof-hedera.js` — self-contained script for the hedera case
- `evm/gateway.js --action submitProof` — submits completed proof to EVM gateway
- `cosmwasm/query.ts multisig-proof` — polls proof status
- `cosmwasm/rotate-signers.js` — similar flow for verifier set rotation
- `evm/docs/amplifier-governance.md` — governance proposal manual proof flow
- `evm/docs/amplifier-its-trusted-chains.md` — ITS trusted chains with manual proof examples
