const { Command } = require('commander');
const { Address, Cell, toNano } = require('@ton/ton');
const { getTonClient, loadWallet, sendTransactionWithCost, waitForTransaction, GATEWAY_ADDRESS } = require('./common');
const { buildTransferOperatorshipMessage } = require('@commonprefix/axelar-cgp-ton');

const TRANSFER_OWNERSHIP_COST = toNano('0.01');

async function run(newOperator) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        const gateway = Address.parse(GATEWAY_ADDRESS);
        const newOperatorAddress = Address.parse(newOperator);
        const transferOwnershipCell = buildTransferOperatorshipMessage(newOperatorAddress);

        var { transfer, seqno } = await sendTransactionWithCost(contract, key, gateway, transferOwnershipCell, TRANSFER_OWNERSHIP_COST);

        console.log('Approve messages transaction sent successfully!');

        await waitForTransaction(contract, seqno);
    } catch (error) {
        console.error('Error in approve messages:', error);
        throw error;
    }
}

// Set up command line interface
if (require.main === module) {
    const program = new Command();
    program
        .name('transferOperatorship')
        .description('Transfer operatorship to another address messages')
        .argument('<newOperator>', 'New operator')
        .action(run);

    program.parse();
}
