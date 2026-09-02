#!/usr/bin/env node
'use strict';

/**
 * Manually construct proof for a stuck axelar -> hedera message.
 *
 * Usage:
 *   MNEMONIC="your axelar mnemonic" node scripts/construct-proof-hedera.js
 *
 * This script:
 *   1. Calls construct_proof on the hedera MultisigProver contract
 *   2. Extracts the multisig_session_id from events
 *   3. Polls until the proof is completed (or times out)
 *   4. Prints the session ID for use with `evm/gateway.js --action submitProof`
 */

const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { GasPrice } = require('@cosmjs/stargate');
const fs = require('fs');
const path = require('path');

// --- Configuration ---
const ENV = 'testnet';
const CONFIG_PATH = path.join(__dirname, '..', 'axelar-chains-config', 'info', `${ENV}.json`);

// The stuck child message (axelar -> hedera)
const SOURCE_CHAIN = 'axelar';
const MESSAGE_ID = '0xf5e570dd157fb4aeeba3415bbfa12219b3f45b8be8bbaf65b005cfa97b4d2c4f-335418567';

const POLL_INTERVAL_MS = 5000;
const POLL_TIMEOUT_MS = 120000;

async function main() {
    const config = JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'));
    const axelar = config.axelar;
    const rpc = axelar.rpc;
    const gasPrice = GasPrice.fromString(axelar.gasPrice);
    const mnemonic = process.env.MNEMONIC;

    if (!mnemonic) {
        console.error('Error: MNEMONIC env var is required (Axelar testnet wallet with some AXL for gas)');
        process.exit(1);
    }

    const multisigProverAddress = axelar.contracts.MultisigProver['hedera']?.address;

    if (!multisigProverAddress) {
        console.error('Error: MultisigProver address for hedera not found in config');
        process.exit(1);
    }

    console.log('=== Construct Proof for Stuck Hedera Message ===');
    console.log('MultisigProver:', multisigProverAddress);
    console.log('Source chain:', SOURCE_CHAIN);
    console.log('Message ID:', MESSAGE_ID);
    console.log('RPC:', rpc);
    console.log('');

    // Create signing client
    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
    const accounts = await wallet.getAccounts();
    console.log('Sender:', accounts[0].address);

    const client = await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice });

    // Check balance
    const balance = await client.getBalance(accounts[0].address, 'uaxl');
    console.log('Balance:', (parseInt(balance.amount) / 1e6).toFixed(6), 'AXL');

    if (parseInt(balance.amount) < 100000) {
        console.error('Error: Insufficient AXL balance. Need at least 0.1 AXL for gas.');
        console.error('Get testnet AXL from: https://faucet.testnet.axelar.dev/');
        process.exit(1);
    }

    console.log('');

    // Step 1: construct_proof
    const msg = {
        construct_proof: [
            {
                source_chain: SOURCE_CHAIN,
                message_id: MESSAGE_ID,
            },
        ],
    };

    console.log('Executing construct_proof...');
    console.log('Message:', JSON.stringify(msg, null, 2));

    let tx;

    try {
        tx = await client.execute(accounts[0].address, multisigProverAddress, msg, 'auto', '');
    } catch (err) {
        console.error('construct_proof failed:', err.message);
        process.exit(1);
    }

    console.log('TX hash:', tx.transactionHash);
    console.log('Gas used:', tx.gasUsed);
    console.log('');

    // Step 2: Extract multisig_session_id
    let sessionId;

    for (const ev of tx.events) {
        if (ev.type === 'wasm-proof_under_construction' || ev.type === 'wasm-signing_started') {
            const attr = ev.attributes.find((a) => a.key === 'multisig_session_id');

            if (attr) {
                // Strip JSON quotes if present (e.g. '"1907448"' -> '1907448')
                sessionId = attr.value.replace(/^"|"$/g, '');
                break;
            }
        }
    }

    if (!sessionId) {
        console.log('Could not find multisig_session_id in events. All events:');

        for (const ev of tx.events) {
            console.log(`  ${ev.type}:`, ev.attributes.map((a) => `${a.key}=${a.value}`).join(', '));
        }

        process.exit(1);
    }

    console.log('=== MULTISIG_SESSION_ID:', sessionId, '===');
    console.log('');

    // Step 3: Poll until completed
    console.log('Polling proof status...');
    const queryClient = await CosmWasmClient.connect(rpc);
    const startTime = Date.now();

    while (Date.now() - startTime < POLL_TIMEOUT_MS) {
        try {
            const result = await queryClient.queryContractSmart(multisigProverAddress, {
                proof: { multisig_session_id: sessionId },
            });

            const status = result.status;

            if (status.completed) {
                console.log('');
                console.log('=== PROOF COMPLETED ===');
                console.log('execute_data length:', status.completed.execute_data.length);
                console.log('');
                console.log('Next step - submit proof to Hedera gateway:');
                console.log(`  PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/gateway.js \\`);
                console.log(`    --action submitProof \\`);
                console.log(`    --multisigSessionId ${sessionId} \\`);
                console.log(`    -n hedera \\`);
                console.log(`    --env testnet \\`);
                console.log(`    -y`);
                return;
            }

            const statusKey = Object.keys(status)[0];
            console.log(`  Status: ${statusKey} (elapsed: ${Math.round((Date.now() - startTime) / 1000)}s)`);
        } catch (err) {
            console.log(`  Query error: ${err.message}`);
        }

        await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    }

    console.log('');
    console.log('Timed out waiting for proof completion.');
    console.log('You can poll manually:');
    console.log(`  ts-node cosmwasm/query.ts multisig-proof hedera ${sessionId} -e testnet`);
    console.log('');
    console.log('Once completed, submit proof:');
    console.log(`  PRIVATE_KEY="$EVM_PRIVATE_KEY" ts-node evm/gateway.js \\`);
    console.log(`    --action submitProof \\`);
    console.log(`    --multisigSessionId ${sessionId} \\`);
    console.log(`    -n hedera \\`);
    console.log(`    --env testnet \\`);
    console.log(`    -y`);
}

main().catch((err) => {
    console.error('Fatal error:', err);
    process.exit(1);
});
