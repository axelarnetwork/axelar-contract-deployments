#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost, GAS_SERVICE_ADDRESS } = require('./common');
const { buildPayNativeGasForContractCallMessage } = require('axelar-cgp-ton');

const program = new Command();
program.name('gasService').description('Axelar TON Gas Service CLI').version('1.0.0');

async function executeOperation(operationName, messageBuilder, cost, args) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);
        const messageBody = messageBuilder(...args);

        const { transfer, seqno } = await sendTransactionWithCost(contract, key, GAS_SERVICE_ADDRESS, messageBody, cost);

        console.log(`✅ ${operationName} transaction sent successfully!`);
        await waitForTransaction(contract, seqno);
    } catch (error) {
        console.error(`❌ Error in ${operationName}:`, error.message);
        process.exit(1);
    }
}

program
    .command('pay-native-gas')
    .description('Pay native TON gas for contract call')
    .argument('<destination-chain>', 'Destination chain name')
    .argument('<destination-address>', 'Destination contract address')
    .argument('<payload>', 'Payload in hex format')
    .argument('<refund-address>', 'Refund address')
    .argument('<gas-amount>', 'Gas amount in TON')
    .action(async (destinationChain, destinationAddress, payload, refundAddress, gasAmount) => {
        const cost = toNano(gasAmount);

        const client = getTonClient();
        const { contract, key } = await loadWallet(client);
        const sender = contract.address;

        await executeOperation(
            'Pay Native Gas',
            (chain, addr, payload, refund) => buildPayNativeGasForContractCallMessage(sender, chain, addr, payload, Address.parse(refund)),
            cost,
            [destinationChain, destinationAddress, payload, refundAddress],
        );
    });

program.parse();
