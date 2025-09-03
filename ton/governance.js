#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address, Cell } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost, bocToCell } = require('./common');
const { buildExecuteTimelockProposalPayload, buildExecuteOperatorProposal } = require('@commonprefix/axelar-cgp-ton');

// Governance contract address - should be set in environment variables
const GOVERNANCE_ADDRESS = process.env.TON_GOVERNANCE_ADDRESS;

if (!GOVERNANCE_ADDRESS) {
    throw new Error('Please set TON_GOVERNANCE_ADDRESS in your .env file');
}

const program = new Command();
program.name('governance').description('Axelar TON Governance CLI').version('1.0.0');

async function executeGovernanceOperation(operationName, messageBody, cost) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        const { transfer, seqno } = await sendTransactionWithCost(contract, key, GOVERNANCE_ADDRESS, messageBody, cost);

        console.log(`✅ ${operationName} transaction sent successfully!`);
        await waitForTransaction(contract, seqno);
    } catch (error) {
        console.error(`❌ Error in ${operationName}:`, error.message);
        process.exit(1);
    }
}

// Execute Timelock Proposal
program
    .command('execute-timelock-proposal')
    .description('Execute a timelock proposal')
    .argument('<target-address>', 'Target contract address')
    .argument('<native-ton>', 'Native TON amount to send with proposal')
    .argument('<proposal-hash>', 'Proposal hash (hex without 0x prefix)')
    .argument('<proposal-hex>', 'Proposal cell data (hex without 0x prefix)')
    .argument('<timelock>', 'Timelock delay in seconds')
    .argument('<actual-timelock>', 'Actual timelock value as bigint')
    .argument('<gas-amount>', 'Gas amount in TON for transaction fees')
    .action(async (targetAddress, nativeTon, proposalHash, proposalHex, timelock, actualTimelock, gasAmount) => {
        try {
            const cost = toNano(gasAmount);

            const proposal = bocToCell(proposalHex);

            // Build the timelock proposal configuration
            const timelockProposal = {
                targetAddress: BigInt('0x' + Address.parse(targetAddress).hash.toString('hex')),
                nativeTon: toNano(nativeTon),
                proposalHash: BigInt('0x' + proposalHash),
                proposal: proposal,
                timelock: toNano(timelock),
                expectedTargetAddress: Address.parse(targetAddress),
            };

            const actualTimelockBigInt = toNano(actualTimelock);

            // Build the payload for timelock proposal execution
            const payload = buildExecuteTimelockProposalPayload(timelockProposal, actualTimelockBigInt);

            await executeGovernanceOperation('Execute Timelock Proposal', payload, cost);
        } catch (error) {
            console.error('❌ Error parsing timelock proposal parameters:', error.message);
            process.exit(1);
        }
    });

// Execute Operator Proposal
program
    .command('execute-operator-proposal')
    .description('Execute an operator proposal')
    .argument('<target-address>', 'Target contract address')
    .argument('<native-ton>', 'Native TON amount to send with proposal')
    .argument('<proposal-hash>', 'Proposal hash (hex without 0x prefix)')
    .argument('<proposal-hex>', 'Proposal cell data (hex without 0x prefix)')
    .argument('<gas-amount>', 'Gas amount in TON for transaction fees')
    .action(async (targetAddress, nativeTon, proposalHash, proposalHex, gasAmount) => {
        try {
            const cost = toNano(gasAmount);

            const proposal = bocToCell(proposalHex);

            // Build the operator proposal configuration
            const operatorProposal = {
                targetAddress: BigInt('0x' + Address.parse(targetAddress).hash.toString('hex')),
                nativeTon: toNano(nativeTon),
                proposalHash: BigInt('0x' + proposalHash),
                proposal: proposal,
            };

            // Build the payload for operator proposal execution
            const payload = buildExecuteOperatorProposal(operatorProposal);

            await executeGovernanceOperation('Execute Operator Proposal', payload, cost);
        } catch (error) {
            console.error('❌ Error parsing operator proposal parameters:', error.message);
            process.exit(1);
        }
    });

program.parse();
