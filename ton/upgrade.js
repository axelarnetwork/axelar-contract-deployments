#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address, beginCell } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost, bocToCell } = require('./common');

// Upgrade operation code
const OP_UPGRADE = 0x00000030;

function buildUpgradeMessage(newCodeHex, newStateHex = null) {
    const newCodeCell = bocToCell(newCodeHex);
    const messageBuilder = beginCell().storeUint(OP_UPGRADE, 32).storeRef(newCodeCell);

    // Add state reference only if provided
    if (newStateHex) {
        const newStateCell = bocToCell(newStateHex);
        messageBuilder.storeRef(newStateCell);
    }

    const message = messageBuilder.endCell();
    return message;
}

async function executeUpgrade(contractAddress, newCodeHex, newStateHex = null) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        const targetContract = Address.parse(contractAddress);
        const messageBody = buildUpgradeMessage(newCodeHex, newStateHex);

        console.log(`🔄 Upgrading contract at: ${contractAddress}`);
        console.log(`🧬 New code size: ${Math.ceil(newCodeHex.length / 2)} bytes`);

        if (newStateHex) {
            console.log(`📊 New state size: ${Math.ceil(newStateHex.length / 2)} bytes`);
        }

        const { transfer, seqno } = await sendTransactionWithCost(contract, key, targetContract, messageBody, toNano('0.2'));

        console.log('✅ Upgrade transaction sent successfully!');
        await waitForTransaction(contract, seqno);

        console.log('🎉 Contract upgrade completed successfully!');
    } catch (error) {
        console.error('❌ Error during contract upgrade:', error.message);
        process.exit(1);
    }
}

// CLI setup - direct arguments without subcommands
const program = new Command();
program
    .name('upgrade')
    .description('Upgrade a TON contract with new code and optionally new state')
    .version('1.0.0')
    .argument('<contract-address>', 'Address of the contract to upgrade')
    .argument('<new-code-hex>', 'New contract code as hex BOC (without 0x prefix)')
    .argument('<new-state-hex>', 'Optional new contract state as hex BOC (without 0x prefix)')
    .action(async (contractAddress, newCodeHex, newStateHex) => {
        await executeUpgrade(contractAddress, newCodeHex, newStateHex);
    });

program.parse();

module.exports = {
    buildUpgradeMessage,
    executeUpgrade,
    OP_UPGRADE,
};
